use super::rest_types::ItineraryCancelRequest;
use crate::grpc::client::GrpcClients;
use axum::{extract::Extension, Json};
use hyper::StatusCode;
use lib_common::uuid::to_uuid;
use svc_scheduler_client_grpc::prelude::scheduler_storage::flight_plan::FlightPriority;
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
    request_body = ItineraryCancelRequest
)]
pub async fn cancel_itinerary(
    Extension(grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<ItineraryCancelRequest>,
) -> Result<(), StatusCode> {
    rest_debug!("(cancel_itinerary) entry.");

    to_uuid(&payload.id).ok_or_else(|| {
        rest_error!("(cancel_itinerary) itinerary ID not in UUID format.");
        StatusCode::BAD_REQUEST
    })?;

    to_uuid(&payload.user_id).ok_or_else(|| {
        rest_error!("(cancel_itinerary) user ID not in UUID format.");
        StatusCode::BAD_REQUEST
    })?;

    // Make request, process response
    #[cfg(not(tarpaulin_include))]
    // no_coverage: need backends to test (integration)
    grpc_clients
        .scheduler
        .cancel_itinerary(svc_scheduler_client_grpc::client::CancelItineraryRequest {
            priority: FlightPriority::Medium as i32,
            itinerary_id: payload.id.clone(),
            user_id: payload.user_id,
        })
        .await
        .map_err(|e| {
            rest_error!("(cancel_itinerary) svc-scheduler request fail. {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rest_info!("(cancel_itinerary) cancellation added to scheduler queue.");

    //
    // Get parcel from id
    //
    let filter =
        AdvancedSearchFilter::search_equals("itinerary_id".to_string(), payload.id.clone());

    #[cfg(not(tarpaulin_include))]
    // no_coverage: need backends to test (integration)
    let futures = grpc_clients
        .storage
        .parcel
        .search(filter)
        .await
        .map_err(|e| {
            rest_error!("(cancel_itinerary) svc-parcel-storage error {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_inner()
        .list
        .into_iter()
        .map(|parcel| async {
            grpc_clients
                .storage
                .parcel
                .delete(Id { id: parcel.id })
                .await
                .map_err(|e| {
                    rest_error!("(cancel_itinerary) svc-storage error: {:?}", e);
                })
        })
        .collect::<Vec<_>>();

    #[cfg(not(tarpaulin_include))]
    // no_coverage: need backends to test (integration)
    {
        if !futures::future::join_all(futures)
            .await
            .into_iter()
            .all(|r| r.is_ok())
        {
            rest_error!("(cancel_itinerary) could not delete all parcels.");
        }
    }

    // If the customer's itinerary was cancelled, but the parcels were not, it's still a success for them
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cancel_itinerary() {
        let config = crate::config::Config::default();
        let grpc_clients = GrpcClients::default(config);

        // invalid itinerary UUID
        let payload = ItineraryCancelRequest {
            id: "".to_string(),
            user_id: "00000000-0000-0000-0000-000000000000".to_string(),
        };

        let result = cancel_itinerary(Extension(grpc_clients.clone()), Json(payload))
            .await
            .unwrap_err();
        assert_eq!(result, StatusCode::BAD_REQUEST);

        // invalid user UUID
        let payload = ItineraryCancelRequest {
            id: "00000000-0000-0000-0000-000000000000".to_string(),
            user_id: "".to_string(),
        };

        let result = cancel_itinerary(Extension(grpc_clients.clone()), Json(payload))
            .await
            .unwrap_err();
        assert_eq!(result, StatusCode::BAD_REQUEST);

        let payload = ItineraryCancelRequest {
            id: "00000000-0000-0000-0000-000000000000".to_string(),
            user_id: "00000000-0000-0000-0000-000000000000".to_string(),
        };

        cancel_itinerary(Extension(grpc_clients), Json(payload))
            .await
            .unwrap();
    }
}
