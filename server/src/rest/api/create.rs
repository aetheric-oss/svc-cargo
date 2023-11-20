pub use super::rest_types::{ItineraryCreateRequest, TaskResponse};
use super::utils::is_uuid;
use crate::cache::DraftItinerary;
use crate::grpc::client::GrpcClients;
use axum::{extract::Extension, Json};
use hyper::StatusCode;
use svc_scheduler_client_grpc::client::CreateItineraryRequest;
use svc_scheduler_client_grpc::prelude::{FlightPriority, SchedulerServiceClient};
use svc_storage_client_grpc::prelude::*;
// use svc_storage_client_grpc::resources::parcel::{Data as ParcelData, ParcelStatus};

/// Confirm an itinerary
/// This will create an itinerary with the scheduler, and will register the parcel with
///  the storage service.
#[utoipa::path(
    put,
    path = "/cargo/create",
    tag = "svc-cargo",
    request_body = ItineraryCreateRequest,
    responses(
        (status = 200, description = "Itinerary created", body = String),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "Microservice dependency returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    )
)]
pub async fn create_itinerary(
    Extension(grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<ItineraryCreateRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    rest_debug!("(create_itinerary) entry.");

    if !is_uuid(&payload.itinerary_id) {
        let error_msg = "flight plan ID not in UUID format.".to_string();
        rest_error!("(create_itinerary) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut itinerary = DraftItinerary::default();

    {
        let Ok(mut pool) = crate::cache::pool::get_pool() else {
            rest_error!("(create_itinerary) unable to get redis pool.");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        let Ok(itinerary) = pool.get_itinerary(payload.itinerary_id.clone()) else {
            rest_error!(
                "(create_itinerary) invalid itinerary id {}",
                payload.itinerary_id
            );
            return Err(StatusCode::BAD_REQUEST);
        };
    }

    //
    // Create itinerary through scheduler
    //

    // Make request, process response
    let data = CreateItineraryRequest {
        flight_plans: itinerary.flight_plans.clone(),
        priority: FlightPriority::Low as i32,
    };

    match grpc_clients.scheduler.create_itinerary(data).await {
        Ok(response) => Ok(Json(response.into_inner())),
        Err(e) => {
            let error_msg = "svc-scheduler error.".to_string();
            rest_error!("(create_itinerary) {} {:?}", &error_msg, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

//     //
//     // Register Parcel with Storage
//     //
//     let itinerary_id = response.id;
//     let data = ParcelData {
//         user_id: payload.user_id,
//         weight_grams: payload.weight_grams,
//         status: ParcelStatus::Notdroppedoff as i32,
//     };

//     // TODO(R4): Push to queue, in case this call fails need a retry mechanism
//     // Make request, process response
//     let response = match grpc_clients.storage.parcel.insert(data).await {
//         Ok(response) => response.into_inner(),
//         Err(e) => {
//             let error_msg = "svc-parcel-storage error.".to_string();
//             rest_error!("(create_itinerary) {} {:?}", &error_msg, e);
//             return Err(StatusCode::INTERNAL_SERVER_ERROR);
//         }
//     };

//     let Some(result) = response.validation_result else {
//         let error_msg = "svc-parcel-storage validation fail.".to_string();
//         rest_error!("(create_itinerary) {}", &error_msg);
//         return Err(StatusCode::INTERNAL_SERVER_ERROR);
//     };

//     let Some(object) = response.object else {
//         let error_msg = "svc-parcel-storage insert fail.".to_string();
//         rest_error!("(create_itinerary) {}", &error_msg);
//         return Err(StatusCode::INTERNAL_SERVER_ERROR);
//     };

//     let parcel_id = object.id;
//     if !result.success {
//         let error_msg = "svc-parcel-storage insert fail.".to_string();
//         rest_error!("(create_itinerary) {}", &error_msg);
//         return Err(StatusCode::INTERNAL_SERVER_ERROR);
//     }

//     Ok(Json(TaskResponse {
//         itinerary_id,
//         parcel_id,
//     }))
