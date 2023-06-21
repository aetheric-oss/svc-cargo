use super::utils::is_uuid;
use crate::grpc::client::GrpcClients;
use crate::rest_types::{ItineraryConfirm, ItineraryConfirmation};
use axum::{extract::Extension, Json};
use hyper::StatusCode;
use svc_scheduler_client_grpc::grpc::ConfirmItineraryRequest;
use svc_storage_client_grpc::resources::parcel::{Data as ParcelData, ParcelStatus};
use svc_storage_client_grpc::ClientConnect;

/// Confirm an itinerary
/// This will confirm an itinerary with the scheduler, and will register the parcel with
///  the storage service.
#[utoipa::path(
    put,
    path = "/cargo/confirm",
    tag = "svc-cargo",
    request_body = ItineraryConfirm,
    responses(
        (status = 200, description = "Itinerary confirmed", body = String),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "Microservice dependency returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    )
)]
pub async fn confirm_itinerary(
    Extension(mut grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<ItineraryConfirm>,
) -> Result<Json<ItineraryConfirmation>, StatusCode> {
    rest_debug!("(confirm_itinerary) entry.");

    if !is_uuid(&payload.id) {
        let error_msg = "flight plan ID not in UUID format.".to_string();
        rest_error!("(confirm_itinerary) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    //
    // Confirm itinerary with scheduler
    //
    let Some(mut client) = grpc_clients.scheduler.get_client().await else {
        let error_msg = "svc-scheduler unavailable.".to_string();
        rest_error!("(confirm_itinerary) {}", &error_msg);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    // Make request, process response
    let data = ConfirmItineraryRequest {
        id: payload.id,
        user_id: payload.user_id,
    };
    let request = tonic::Request::new(data);
    let response = match client.confirm_itinerary(request).await {
        Ok(response) => response.into_inner(),
        Err(e) => {
            let error_msg = "svc-scheduler error.".to_string();
            rest_error!("(confirm_itinerary) {} {:?}", &error_msg, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if !response.confirmed {
        let error_msg = "svc-scheduler confirm fail.".to_string();
        rest_error!("(confirm_itinerary) {}", &error_msg);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    //
    // Register Parcel with Storage
    //
    let itinerary_id = response.id;
    let data = ParcelData {
        itinerary_id: itinerary_id.clone(),
        status: ParcelStatus::Notdroppedoff as i32,
    };

    // TODO(R4): Push to queue, in case this call fails need a retry mechanism
    let request = tonic::Request::new(data);
    let Ok(mut client) = grpc_clients.storage.parcel.get_client().await else {
        let error_msg = "svc-parcel-storage unavailable.".to_string();
        rest_error!("(confirm_itinerary) {}", &error_msg);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    // Make request, process response
    let response = match client.insert(request).await {
        Ok(response) => response.into_inner(),
        Err(e) => {
            let error_msg = "svc-parcel-storage error.".to_string();
            rest_error!("(confirm_itinerary) {} {:?}", &error_msg, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let Some(result) = response.validation_result else {
        let error_msg = "svc-parcel-storage validation fail.".to_string();
        rest_error!("(confirm_itinerary) {}", &error_msg);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let Some(object) = response.object else {
        let error_msg = "svc-parcel-storage insert fail.".to_string();
        rest_error!("(confirm_itinerary) {}", &error_msg);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let parcel_id = object.id;
    if !result.success {
        let error_msg = "svc-parcel-storage insert fail.".to_string();
        rest_error!("(confirm_itinerary) {}", &error_msg);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json(ItineraryConfirmation {
        itinerary_id,
        parcel_id,
    }))
}
