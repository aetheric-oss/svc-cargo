use super::utils::is_uuid;
use crate::grpc::client::GrpcClients;
use crate::rest_types::ItineraryCancel;
use axum::{extract::Extension, Json};
use hyper::StatusCode;
use svc_scheduler_client_grpc::grpc::Id as ResourceId;
use svc_storage_client_grpc::{AdvancedSearchFilter, ClientConnect, Id};

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
    Extension(mut grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<ItineraryCancel>,
) -> Result<(), StatusCode> {
    rest_debug!("(cancel_itinerary) entry.");
    let itinerary_id = payload.id;
    if !is_uuid(&itinerary_id) {
        let error_msg = "itinerary ID not in UUID format.".to_string();
        rest_error!("(cancel_itinerary) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Get Client
    let client_option = grpc_clients.scheduler.get_client().await;
    let Some(mut client) = client_option else {
        let error_msg = "svc-scheduler unavailable.".to_string();
        rest_error!("(cancel_itinerary) {}", &error_msg);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    // Make request, process response
    let request = tonic::Request::new(ResourceId {
        id: itinerary_id.clone(),
    });
    let response = match client.cancel_itinerary(request).await {
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
    let Ok(mut client) = grpc_clients.storage.parcel.get_client().await else {
        let error_msg = "svc-parcel-storage unavailable.".to_string();
        rest_error!("(cancel_itinerary) {}", &error_msg);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let filter =
        AdvancedSearchFilter::search_equals("itinerary_id".to_string(), itinerary_id.clone());

    let list = match client.search(filter).await {
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
    for parcel in list {
        let request = tonic::Request::new(Id { id: parcel.id });
        match client.delete(request).await {
            Ok(_) => {
                // Delete activity currently returns Empty
                // response.into_inner()
            }
            Err(e) => {
                let error_msg = "svc-parcel-storage error.".to_string();
                rest_error!("(cancel_itinerary) {} {:?}", &error_msg, e);
                // Still try to delete other parcels
                ok = false;
            }
        };
    }

    if !ok {
        rest_error!("(cancel_itinerary) could not delete all parcels.");
    }

    // If the customer's itinerary was cancelled, but the parcels were not, it's still a success for them
    Ok(())
}
