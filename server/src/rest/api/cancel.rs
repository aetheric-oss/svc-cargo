use super::rest_types::ItineraryCancel;
use super::utils::is_uuid;
use crate::grpc::client::GrpcClients;
use axum::{extract::Extension, Json};
use hyper::StatusCode;
use svc_scheduler_client_grpc::prelude::SchedulerServiceClient;
use svc_storage_client_grpc::prelude::*;

/// Cancel a Flight
#[utoipa::path(
    delete,
    path = "/cargo/cancel",
    tag = "svc-cargo",
    responses(
        (status = 200, description = "Flight cancelled successfully"),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "svc-scheduler returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    ),
    request_body = ItineraryCancel
)]
pub async fn cancel_itinerary(
    Extension(grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<ItineraryCancel>,
) -> Result<(), StatusCode> {
    rest_debug!("(cancel_itinerary) entry.");
    let itinerary_id = payload.id;
    if !is_uuid(&itinerary_id) {
        let error_msg = "itinerary ID not in UUID format.".to_string();
        rest_error!("(cancel_itinerary) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Make request, process response
    let response = match grpc_clients
        .scheduler
        .cancel_itinerary(svc_scheduler_client_grpc::client::Id {
            id: itinerary_id.clone(),
        })
        .await
    {
        Ok(response) => response.into_inner(),
        Err(e) => {
            let error_msg = "svc-scheduler request fail.".to_string();
            rest_error!("(cancel_itinerary) {} {:?}", &error_msg, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if !response.cancelled {
        let error_msg = "svc-scheduler cancel fail.".to_string();
        rest_error!("(cancel_itinerary) {} {}", &error_msg, response.reason);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    rest_info!("(cancel_itinerary) successfully cancelled itinerary.");

    //
    // Get parcel from id
    //
    let filter =
        AdvancedSearchFilter::search_equals("itinerary_id".to_string(), itinerary_id.clone());

    let list = match grpc_clients.storage.parcel.search(filter).await {
        Ok(response) => response.into_inner().list,
        Err(e) => {
            let error_msg = "svc-parcel-storage error.".to_string();
            rest_error!("(cancel_itinerary) {} {:?}", &error_msg, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // An itinerary might have multiple parcels
    // TODO(R4): Push these onto a queue in case any one fails
    let mut ok = true;
    for parcel in list.into_iter() {
        let _ = grpc_clients
            .storage
            .parcel
            .delete(Id { id: parcel.id })
            .await
            .map_err(|e| {
                let error_msg = "svc-parcel-storage error.".to_string();
                rest_error!("(cancel_itinerary) {} {:?}", &error_msg, e);
                // Still try to delete other parcels
                ok = false;
            });
    }

    if !ok {
        rest_error!("(cancel_itinerary) could not delete all parcels.");
    }

    // If the customer's itinerary was cancelled, but the parcels were not, it's still a success for them
    Ok(())
}
