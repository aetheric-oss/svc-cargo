pub use super::rest_types::{
    CargoInfo, CurrencyUnit, Itinerary, ItineraryCreateRequest, SchedulerFlightPlan,
};
use crate::cache::pool::ItineraryPool;
use crate::grpc::client::GrpcClients;
use axum::{extract::Extension, Json};
use chrono::{DateTime, Duration, Utc};
use hyper::StatusCode;
use num_traits::FromPrimitive;
use svc_scheduler_client_grpc::client::{
    CreateItineraryRequest, TaskRequest, TaskResponse, TaskStatus, TaskStatusRationale,
};
use svc_scheduler_client_grpc::prelude::scheduler_storage::flight_plan::FlightPriority;
use svc_scheduler_client_grpc::prelude::SchedulerServiceClient;
use svc_storage_client_grpc::link_service::Client as LinkClient;
use svc_storage_client_grpc::prelude::flight_plan_parcel::RowData as FlightPlanParcel;
use svc_storage_client_grpc::prelude::AdvancedSearchFilter;
use svc_storage_client_grpc::prelude::Id as StorageId;
use svc_storage_client_grpc::simple_service::Client as SimpleClient;
use svc_storage_client_grpc::simple_service_linked::Client as SimpleLinkedClient;

/// Polling interval for scheduler task statuses
const SCHEDULER_TASK_POLL_INTERVAL_SECONDS: u64 = 5;

/// Timeout for scheduler task statuses
const SCHEDULER_TASK_TIMEOUT_SECONDS: i64 = 60;

// use svc_storage_client_grpc::resources::itinerary;
use svc_storage_client_grpc::resources::parcel::{Data as ParcelData, ParcelStatus};

///
/// Charge the customer for the itinerary
///  If 'dry_run' is true, only check the validity of the
///  payment option first.
async fn payment_confirm(
    // TODO(R5): user credential or UUID
    _total: f32,
    _currency_unit: CurrencyUnit,
    dry_run: bool,
) -> Result<(), StatusCode> {
    rest_debug!("(payment_method_check) entry.");
    //
    // TODO(R5): Check if payment options are valid
    //
    // Possibly query storage for payment information
    // Credit Card, ACH, Cryptocurrency Wallet, etc.
    // In the case of crypto, verify wallet has sufficient
    // funds

    if dry_run {
        return Ok(());
    }

    //
    // TODO(R5): payment service confirm
    // If payment doesn't work here for some reason,
    //  add a scheduler task to cancel the itinerary

    Ok(())
}

/// Pull the itinerary details from Redis
async fn get_draft_itinerary(itinerary_id: &str) -> Result<Itinerary, StatusCode> {
    rest_debug!("(get_draft_itinerary) getting itinerary from redis.");

    let Some(mut pool) = crate::cache::pool::get_pool().await else {
        rest_error!("(get_draft_itinerary) unable to get redis pool.");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    pool.get_itinerary(itinerary_id.to_string())
        .await
        .map_err(|e| {
            rest_error!("(get_draft_itinerary) unable to get itinerary from redis: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Make a request to the scheduler to create an itinerary
async fn scheduler_request(
    itinerary: &Itinerary,
    expiry: DateTime<Utc>,
    grpc_clients: &GrpcClients,
) -> Result<TaskResponse, StatusCode> {
    rest_debug!("(scheduler_request) creating itinerary with scheduler.");

    let Ok(flight_plans) = itinerary
        .flight_plans
        .clone()
        .into_iter()
        .map(|fp| fp.try_into())
        .collect::<Result<Vec<SchedulerFlightPlan>, _>>()
    else {
        rest_error!("(scheduler_request) invalid flight plan data.");
        return Err(StatusCode::BAD_REQUEST);
    };

    let data = CreateItineraryRequest {
        flight_plans,
        priority: FlightPriority::Low as i32,
        expiry: Some(expiry.into()),
        user_id: itinerary.user_id.clone(),
    };

    grpc_clients
        .scheduler
        .create_itinerary(data)
        .await
        .map_err(|e| {
            let error_msg = "svc-scheduler error.".to_string();
            rest_error!("(scheduler_request) {} {:?}", &error_msg, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
        .map(|response| response.into_inner())
}

/// Poll the scheduler for the task status for a set amount of time
async fn scheduler_poll(
    task_id: i64,
    expiry: DateTime<Utc>,
    grpc_clients: GrpcClients,
) -> Result<String, StatusCode> {
    rest_debug!("(scheduler_poll) polling scheduler for task status.");

    // Poll scheduler every few seconds
    let interval = tokio::time::Duration::from_secs(SCHEDULER_TASK_POLL_INTERVAL_SECONDS);
    let request = TaskRequest { task_id };

    // TODO(R4): Tasks from svc-cargo should expire after N seconds
    //  Provide expiry in request to the scheduler.
    while Utc::now() < expiry {
        // give the scheduler time to process the request
        tokio::time::sleep(interval).await;

        let task = match grpc_clients.scheduler.get_task_status(request).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                let error_msg = "svc-scheduler error.".to_string();
                rest_error!("(create_itinerary) {} {:?}", &error_msg, e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

        let Some(metadata) = task.task_metadata else {
            rest_error!("(create_itinerary) no metadata for task: {:?}", task);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        let Some(status) = FromPrimitive::from_i32(metadata.status) else {
            rest_error!(
                "(create_itinerary) unrecognized task status: {:?}",
                metadata.status
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        match status {
            TaskStatus::Queued => {
                // Do nothing
            }
            TaskStatus::Complete => return Ok(metadata.result.unwrap_or("".to_string())),
            TaskStatus::NotFound => {
                rest_error!("(create_itinerary) svc-scheduler error.");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            TaskStatus::Rejected => {
                rest_warn!(
                    "(create_itinerary) task was rejected by the scheduler: {}",
                    metadata
                        .status_rationale
                        .unwrap_or(TaskStatusRationale::InvalidAction as i32)
                );
                return Err(StatusCode::NOT_MODIFIED);
            }
        }
    }

    rest_warn!("(create_itinerary) task timed out.");

    // Fire off task cancellation and don't wait
    tokio::spawn(async move {
        let _ = grpc_clients.scheduler.cancel_task(request).await;
    });

    Err(StatusCode::REQUEST_TIMEOUT)
}

/// Create the parcel/book the seat
async fn create_cargo(
    itinerary: &Itinerary,
    itinerary_id: &str,
    acquisition_vertiport_id: &str,
    delivery_vertiport_id: &str,
    grpc_clients: &GrpcClients,
) -> Result<CargoInfo, StatusCode> {
    //
    // TODO(R5): Doing all of these in a transaction would be
    //  nice, to rollback any changes if there are errors at
    //  any point. For now we'll have orphaned records if there
    //  are issues.
    //

    ///////////////////////////////
    // Register Parcel with Storage
    ///////////////////////////////
    let data = ParcelData {
        user_id: itinerary.user_id.clone(),
        weight_grams: itinerary.cargo_weight_g,
        status: ParcelStatus::Notdroppedoff as i32,
    };

    // TODO(R4): Push to queue, in case this call fails need a retry mechanism
    // Make request, process response
    let object = grpc_clients
        .storage
        .parcel
        .insert(data)
        .await
        .map_err(|e| {
            let error_msg = "svc-parcel-storage insert fail.".to_string();
            rest_error!("(create_itinerary) {} {:?}", &error_msg, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_inner()
        .object
        .ok_or_else(|| {
            let error_msg = "svc-parcel-storage insert fail.".to_string();
            rest_error!("(create_itinerary) {}", &error_msg);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let cargo_id = object.id;

    ///////////////////////////////
    // Get the itinerary and flight plans just created for user
    ///////////////////////////////

    //
    // TODO(R5): this is a bit hacky. with the asynchronous task-based scheduler approach,
    //  maybe return the itinerary id in the TaskResponse in the future
    let filter =
        AdvancedSearchFilter::search_equals("itinerary_id".to_string(), itinerary_id.to_string());

    let db_itinerary = grpc_clients
        .storage
        .itinerary
        .search(filter)
        .await
        .map_err(|e| {
            let error_msg = "error on request to svc-storage.".to_string();
            rest_error!("(create_itinerary) {} {:?}", &error_msg, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_inner()
        .list
        .pop()
        .ok_or_else(|| {
            let error_msg = "svc-storage error, no itineraries found.".to_string();
            rest_error!("(create_itinerary) {}", &error_msg);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    //
    // Get the linked flight plans
    // Need the IDs of the flight plans to update the flight_plan_parcel table
    let flight_plans = grpc_clients
        .storage
        .itinerary_flight_plan_link
        .get_linked(StorageId {
            id: db_itinerary.id,
        })
        .await
        .map_err(|e| {
            let error_msg = "error on request to svc-storage.".to_string();
            rest_error!("(create_itinerary) {} {:?}", &error_msg, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_inner()
        .list;

    //
    // Acquisition flight plan
    //  The itinerary itself has no knowledge of what the "start point" is for
    //  a package. Could be the first flight, or the second flight (with
    //  the first being deadhead). Similarly, the delivery flight
    //  could be the last or second to last flight plan.
    //
    // We could maybe indicate which flight(s) are the acquisition and delivery
    //  in the itinerary record itself, so we don't have to search here.

    for fp in flight_plans {
        let Some(ref data) = fp.data else {
            let error_str = "flight plan data not found.".to_string();
            rest_error!("(create_itinerary) {}", &error_str);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        let origin_vertiport_id = match data.origin_vertiport_id {
            Some(ref id) => id.clone(),
            None => {
                let filter = AdvancedSearchFilter::search_equals(
                    "vertipad_id".to_string(),
                    data.origin_vertipad_id.clone(),
                );

                grpc_clients
                    .storage
                    .vertipad
                    .search(filter)
                    .await
                    .map_err(|e| {
                        let error_msg = "svc-storage error searching vertipad.".to_string();
                        rest_error!("(create_itinerary) {} {:?}", &error_msg, e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?
                    .into_inner()
                    .list
                    .pop()
                    .ok_or_else(|| {
                        let error_msg = "vertipad not found.".to_string();
                        rest_error!("(create_itinerary) {}", &error_msg);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?
                    .data
                    .ok_or_else(|| {
                        let error_msg = "vertipad data not found.".to_string();
                        rest_error!("(create_itinerary) {}", &error_msg);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?
                    .vertiport_id
            }
        };

        let target_vertiport_id = match data.target_vertiport_id {
            Some(ref id) => id.clone(),
            None => {
                let filter = AdvancedSearchFilter::search_equals(
                    "vertipad_id".to_string(),
                    data.target_vertipad_id.clone(),
                );

                grpc_clients
                    .storage
                    .vertipad
                    .search(filter)
                    .await
                    .map_err(|e| {
                        let error_msg = "svc-storage error searching vertipad.".to_string();
                        rest_error!("(create_itinerary) {} {:?}", &error_msg, e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?
                    .into_inner()
                    .list
                    .pop()
                    .ok_or_else(|| {
                        let error_msg = "vertipad not found.".to_string();
                        rest_error!("(create_itinerary) {}", &error_msg);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?
                    .data
                    .ok_or_else(|| {
                        let error_msg = "vertipad data not found.".to_string();
                        rest_error!("(create_itinerary) {}", &error_msg);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?
                    .vertiport_id
            }
        };

        if origin_vertiport_id == acquisition_vertiport_id {
            let acquisition = FlightPlanParcel {
                flight_plan_id: fp.id.clone(),
                parcel_id: cargo_id.clone(),
                acquire: true,
                deliver: false,
            };

            let _ = grpc_clients
                .storage
                .flight_plan_parcel
                .insert(acquisition)
                .await
                .map_err(|e| {
                    let error_msg =
                        "svc-storage error inserting flight_plan_parcel link (acquisition)."
                            .to_string();
                    rest_error!("(create_itinerary) {} {:?}", &error_msg, e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }

        if target_vertiport_id == delivery_vertiport_id {
            let delivery = FlightPlanParcel {
                flight_plan_id: fp.id.clone(),
                parcel_id: cargo_id.clone(),
                acquire: false,
                deliver: true,
            };

            let _ = grpc_clients
                .storage
                .flight_plan_parcel
                .insert(delivery)
                .await
                .map_err(|e| {
                    let error_msg =
                        "svc-storage error inserting flight_plan_parcel link (delivery)."
                            .to_string();
                    rest_error!("(create_itinerary) {} {:?}", &error_msg, e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
    }

    Ok(CargoInfo {
        cargo_id,
        // cargo_nickname
    })
}

/// Confirm an itinerary
/// This will create an itinerary with the scheduler, and will register the parcel with
///  the storage service.
#[utoipa::path(
    put,
    path = "/cargo/create",
    tag = "svc-cargo",
    request_body = ItineraryCreateRequest,
    responses(
        (status = 200, description = "Itinerary created."),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "Microservice dependency returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    )
)]
pub async fn create_itinerary(
    Extension(grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<ItineraryCreateRequest>,
) -> Result<(), StatusCode> {
    rest_debug!("(create_itinerary) entry.");

    //
    // See if itinerary id exists
    let itinerary = get_draft_itinerary(&payload.id).await?;
    let invoice_total = itinerary.invoice.iter().map(|i| i.cost).sum::<f32>();

    //
    // Check if payment options are valid/sufficient funds
    //  in the case of cryptocurrency
    payment_confirm(
        invoice_total,
        itinerary.currency_unit,
        true, // dry run, don't charge the customer
    )
    .await?;

    //
    // Ask the scheduler to attempt to create the itinerary
    // This will "reserve" the weight/seats as well so we can
    //  create the parcel record in storage later without conflicts.
    let delta = Duration::try_seconds(SCHEDULER_TASK_TIMEOUT_SECONDS).ok_or_else(|| {
        rest_error!("(create_itinerary) failed to create duration.");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let expiry = Utc::now() + delta;
    let task_id = scheduler_request(&itinerary, expiry, &grpc_clients)
        .await?
        .task_id;

    //
    // Poll the scheduler for the task status for a set amount of time
    let itinerary_id = scheduler_poll(task_id, expiry, grpc_clients.clone()).await?;
    uuid::Uuid::try_parse(&itinerary_id).map_err(|e| {
        rest_error!("(create_itinerary) invalid itinerary ID returned: {itinerary_id} {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    //
    // Create the parcel/book the seat
    //
    let _ = create_cargo(
        &itinerary,
        &itinerary_id,
        &itinerary.acquisition_vertiport_id,
        &itinerary.delivery_vertiport_id,
        &grpc_clients,
    )
    .await?;

    //
    // If the scheduler task was successful, charge the customer
    //
    payment_confirm(invoice_total, itinerary.currency_unit, false).await?;

    Ok(())
}
