pub use super::rest_types::{
    CargoInfo, CurrencyUnit, Itinerary, ItineraryCreateRequest, SchedulerFlightPlan,
};
use crate::cache::pool::ItineraryPool;
use crate::grpc::client::GrpcClients;
use axum::{extract::Extension, Json};
use hyper::StatusCode;
use lib_common::time::{DateTime, Duration, Utc};
use lib_common::uuid::to_uuid;
use num_traits::FromPrimitive;
use svc_contact_client_grpc::client::CargoConfirmationRequest;
use svc_contact_client_grpc::prelude::ContactServiceClient;
use svc_scheduler_client_grpc::client::{
    CreateItineraryRequest, TaskRequest, TaskResponse, TaskStatus, TaskStatusRationale,
};
use svc_scheduler_client_grpc::prelude::scheduler_storage::flight_plan::FlightPriority;
use svc_scheduler_client_grpc::prelude::SchedulerServiceClient;
use svc_storage_client_grpc::link_service::Client as LinkClient;
use svc_storage_client_grpc::prelude::flight_plan_parcel::RowData as FlightPlanParcel;
use svc_storage_client_grpc::prelude::Id as StorageId;
use svc_storage_client_grpc::simple_service::Client as SimpleClient;
use svc_storage_client_grpc::simple_service_linked::Client as SimpleLinkedClient;

/// Polling interval for scheduler task statuses
const SCHEDULER_TASK_POLL_INTERVAL_SECONDS: u64 = 3;

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
    rest_debug!("entry.");
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

/// Make a request to the scheduler to create an itinerary
#[cfg(not(tarpaulin_include))]
// no_coverage: (R5) need backends to test (integration)
async fn scheduler_request(
    itinerary: &Itinerary,
    expiry: DateTime<Utc>,
    grpc_clients: &GrpcClients,
) -> Result<TaskResponse, StatusCode> {
    rest_debug!("creating itinerary with scheduler.");

    let flight_plans = itinerary
        .flight_plans
        .clone()
        .into_iter()
        .map(|fp| fp.try_into())
        .collect::<Result<Vec<SchedulerFlightPlan>, _>>()
        .map_err(|e| {
            rest_error!("invalid flight plan data: {e}");
            StatusCode::BAD_REQUEST
        })?;

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
            rest_error!("svc-scheduler error {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
        .map(|response| response.into_inner())
}

/// Poll the scheduler for the task status for a set amount of time
#[cfg(not(tarpaulin_include))]
// no_coverage: need backends to test (integration)
async fn scheduler_poll(
    task_id: i64,
    expiry: DateTime<Utc>,
    grpc_clients: GrpcClients,
) -> Result<String, StatusCode> {
    rest_debug!("polling scheduler for task status.");

    // Poll scheduler every few seconds
    let interval = tokio::time::Duration::from_secs(SCHEDULER_TASK_POLL_INTERVAL_SECONDS);
    let request = TaskRequest { task_id };

    //  Provide expiry in request to the scheduler.
    while Utc::now() < expiry {
        // give the scheduler time to process the request
        tokio::time::sleep(interval).await;

        let task = grpc_clients
            .scheduler
            .get_task_status(request)
            .await
            .map_err(|e| {
                rest_error!("svc-scheduler error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .into_inner();

        let task_id = task.task_id;
        let metadata = task.task_metadata.ok_or_else(|| {
            rest_error!("no metadata for task #{task_id}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let status = FromPrimitive::from_i32(metadata.status).ok_or_else(|| {
            rest_error!("unrecognized task status: {:?}", metadata.status);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        match status {
            TaskStatus::Queued => {
                // Do nothing
            }
            TaskStatus::Complete => return Ok(metadata.result.unwrap_or("".to_string())),
            TaskStatus::NotFound => {
                rest_error!("svc-scheduler error.");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            TaskStatus::Rejected => {
                rest_warn!(
                    "task was rejected by the scheduler: {}",
                    metadata
                        .status_rationale
                        .unwrap_or(TaskStatusRationale::InvalidAction as i32)
                );
                return Err(StatusCode::NOT_MODIFIED);
            }
        }
    }

    rest_warn!("task timed out.");

    // Fire off task cancellation and don't wait
    tokio::spawn(async move {
        let _ = grpc_clients.scheduler.cancel_task(request).await;
    });

    Err(StatusCode::REQUEST_TIMEOUT)
}

/// Create the parcel/book the seat
#[cfg(not(tarpaulin_include))]
// no_coverage: need backends to test (integration)
async fn create_cargo(
    itinerary: &Itinerary,
    itinerary_id: &str,
    acquisition_vertiport_id: &str,
    delivery_vertiport_id: &str,
    grpc_clients: &GrpcClients,
) -> Result<CargoInfo, StatusCode> {
    rest_debug!("creating parcel for itinerary_id {itinerary_id}: acquisition_vertiport_id: {acquisition_vertiport_id}, delivery_vertiport_id: {delivery_vertiport_id}.");
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

    // TODO(R5): Push to queue, in case this call fails need a retry mechanism
    // Make request, process response
    let object = grpc_clients
        .storage
        .parcel
        .insert(data)
        .await
        .map_err(|e| {
            let error_msg = "svc-parcel-storage insert fail.".to_string();
            rest_error!("{} {:?}", &error_msg, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_inner()
        .object
        .ok_or_else(|| {
            let error_msg = "svc-parcel-storage insert fail.".to_string();
            rest_error!("{}", &error_msg);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let parcel_id = object.id;

    //
    // Get the linked flight plans
    // Need the IDs of the flight plans to update the flight_plan_parcel table
    let flight_plans = grpc_clients
        .storage
        .itinerary_flight_plan_link
        .get_linked(StorageId {
            id: itinerary_id.to_string(),
        })
        .await
        .map_err(|e| {
            let error_msg = "error on request to svc-storage.".to_string();
            rest_error!("{} {:?}", &error_msg, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_inner()
        .list;

    if flight_plans.is_empty() {
        let error_str = "no flight plans found for itinerary_id.".to_string();
        rest_error!("{}", &error_str);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    //
    // Acquisition flight plan
    //  The itinerary itself has no knowledge of what the "start point" is for
    //  a package. Could be the first flight, or the second flight (with
    //  the first being deadhead). Similarly, the delivery flight
    //  could be the last or second to last flight plan.
    //
    // We could maybe indicate which flight(s) are the acquisition and delivery
    //  in the itinerary record itself, so we don't have to search here.

    let mut fps: Vec<(String, String, String)> = vec![];
    for fp in flight_plans.into_iter() {
        let data = fp.data.ok_or_else(|| {
            let error_str = "flight plan data not found.".to_string();
            rest_error!("{}", &error_str);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let origin_vertiport_id = match data.origin_vertiport_id {
            Some(ref id) => id.clone(),
            None => {
                super::utils::get_vertiport_id_from_vertipad_id(
                    grpc_clients,
                    &data.origin_vertipad_id,
                )
                .await?
            }
        };

        let target_vertiport_id = match data.target_vertiport_id {
            Some(ref id) => id.clone(),
            None => {
                super::utils::get_vertiport_id_from_vertipad_id(
                    grpc_clients,
                    &data.target_vertipad_id,
                )
                .await?
            }
        };

        fps.push((fp.id, origin_vertiport_id, target_vertiport_id));
    }

    let acquisition_id = &fps
        .iter()
        .find(|fp| fp.1 == acquisition_vertiport_id)
        .ok_or_else(|| {
            let error_str = "acquisition flight plan not found.".to_string();
            rest_error!("{}", &error_str);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .0;

    let delivery_id = &fps
        .iter()
        .find(|fp| fp.2 == delivery_vertiport_id)
        .ok_or_else(|| {
            let error_str = "delivery flight plan not found.".to_string();
            rest_error!("{}", &error_str);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .0;

    let records = if acquisition_id == delivery_id {
        vec![FlightPlanParcel {
            flight_plan_id: acquisition_id.clone(),
            parcel_id: parcel_id.clone(),
            acquire: true,
            deliver: true,
        }]
    } else {
        vec![
            FlightPlanParcel {
                flight_plan_id: acquisition_id.clone(),
                parcel_id: parcel_id.clone(),
                acquire: true,
                deliver: false,
            },
            FlightPlanParcel {
                flight_plan_id: delivery_id.clone(),
                parcel_id: parcel_id.clone(),
                acquire: false,
                deliver: true,
            },
        ]
    };

    for parcel_record in records {
        grpc_clients
            .storage
            .flight_plan_parcel
            .insert(parcel_record)
            .await
            .map_err(|e| {
                let error_msg = "svc-storage error inserting flight_plan_parcel link.".to_string();
                rest_error!("{} {:?}", &error_msg, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    Ok(CargoInfo { parcel_id })
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
#[cfg(not(tarpaulin_include))]
// no_coverage: need backends to test (integration)
pub async fn create_itinerary(
    Extension(grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<ItineraryCreateRequest>,
) -> Result<(), StatusCode> {
    rest_debug!("entry.");

    to_uuid(&payload.id).ok_or_else(|| {
        rest_error!("invalid itinerary UUID.");
        StatusCode::BAD_REQUEST
    })?;

    to_uuid(&payload.user_id).ok_or_else(|| {
        rest_error!("invalid user UUID.");
        StatusCode::BAD_REQUEST
    })?;

    //
    // See if itinerary id exists
    let itinerary = crate::cache::pool::get_pool()
        .await
        .map_err(|e| {
            rest_error!("unable to get redis pool: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .lock()
        .await
        .get_itinerary(payload.id.to_string())
        .await
        .map_err(|e| {
            rest_error!("unable to get itinerary from redis: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

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
        rest_error!("failed to create duration.");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let expiry = Utc::now() + delta;
    let task_id = scheduler_request(&itinerary, expiry, &grpc_clients)
        .await?
        .task_id;

    //
    // Poll the scheduler for the task status for a set amount of time
    let itinerary_id = scheduler_poll(task_id, expiry, grpc_clients.clone()).await?;
    to_uuid(&itinerary_id).ok_or_else(|| {
        rest_error!("invalid itinerary UUID.");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    //
    // Create the parcel/book the seat
    //
    let cargo_data = create_cargo(
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

    // Continue even if the contact service fails
    let data = CargoConfirmationRequest {
        parcel_id: cargo_data.parcel_id,
        itinerary_id,
    };

    let _ = grpc_clients
        .contact
        .cargo_confirmation(data)
        .await
        .map_err(|e| {
            let error_msg = "svc-contact error.".to_string();
            rest_error!("{} {:?}", &error_msg, e);
        });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_common::uuid::Uuid;

    #[tokio::test]
    async fn test_scheduler_poll() {
        let config = crate::config::Config::default();
        let grpc_clients = GrpcClients::default(config);
        let task_id = 12345;

        // will timeout
        let expiry = Utc::now() - Duration::seconds(1);
        let result = scheduler_poll(task_id, expiry, grpc_clients.clone())
            .await
            .unwrap_err();
        assert_eq!(result, StatusCode::REQUEST_TIMEOUT);

        // TODO: tell svc-scheduler to fail the task

        // will complete
        let expiry = Utc::now() + Duration::seconds(1);
        let _ = scheduler_poll(task_id, expiry, grpc_clients.clone())
            .await
            .unwrap();
        // assert_eq!(result, StatusCode::REQUEST_TIMEOUT);
    }

    #[tokio::test]
    async fn test_payment_confirm() {
        let total = 100.0;
        let currency_unit = CurrencyUnit::Usd;
        let dry_run = true;

        let result = payment_confirm(total, currency_unit, dry_run).await;
        assert!(result.is_ok());

        let dry_run = false;
        let result = payment_confirm(total, currency_unit, dry_run).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_itinerary() {
        let config = crate::config::Config::default();
        let grpc_clients = GrpcClients::default(config);

        // bad itinerary id
        let mut request = ItineraryCreateRequest {
            id: "invalid".to_string(),
            user_id: Uuid::new_v4().to_string(),
        };
        let error = create_itinerary(Extension(grpc_clients.clone()), Json(request.clone()))
            .await
            .unwrap_err();
        assert_eq!(error, StatusCode::BAD_REQUEST);

        // bad user id
        request.id = Uuid::new_v4().to_string();
        request.user_id = "invalid".to_string();
        let error = create_itinerary(Extension(grpc_clients.clone()), Json(request.clone()))
            .await
            .unwrap_err();
        assert_eq!(error, StatusCode::BAD_REQUEST);
    }
}
