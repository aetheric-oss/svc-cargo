//! Public API for the svc-cargo server
//! REST Endpoints for client applications

// use std::{
//     sync::Arc,
//     // collections::HashMap
// };

use axum::{response::IntoResponse, Json};
use hyper::{HeaderMap, StatusCode};
mod types {
    include!("../../openapi/types.rs");
}
pub use types::*;

/// Get All Vertiports in a Region
///
/// List all Vertiport items from svc-storage
#[utoipa::path(
    get,
    path = "/cargo/vertiports",
    request_body = VertiportsQuery,
    responses(
        (status = 200, description = "List all cargo-accessible vertiports successfully", body = [Vertiport])
    )
)]
pub async fn query_vertiports(Json(payload): Json<VertiportsQuery>) -> Json<Vec<Vertiport>> {
    // let vertiports = store.lock().await.clone();
    // TODO Query svc-storage
    println!("{:?}", payload);

    Json(vec![])
}

/// Search FlightOptions by query params.
///
/// Search `FlightOption`s by query params and return matching `FlightOption`s.
#[utoipa::path(
    get,
    path = "/cargo/query",
    request_body = FlightQuery,
    responses(
        (status = 200, description = "List possible flights", body = [FlightOption])
    )
)]
pub async fn query_flight(Json(payload): Json<FlightQuery>) -> Json<Vec<FlightOption>> {
    // TODO get from svc-storage
    // Validate query fields
    //    lib-common: validate_fp_id
    println!("{:?}", payload);

    Json(vec![])
}

/// Confirm a Flight
///
/// Tries to confirm a flight with the svc-scheduler
#[utoipa::path(
    put,
    path = "/cargo/confirm",
    request_body = FlightConfirm,
    responses(
        (status = 201, description = "Flight Confirmed", body = String),
        (status = 409, description = "Flight Confirmation Failed", body = ConfirmError)
    ),
    security(
        (), // <-- make optional authentication
        ("api_key" = [])
    )
)]
pub async fn confirm_flight(
    Json(payload): Json<FlightConfirm>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    // TODO Confirm with svc-scheduler
    // Get Result
    // .unwrap_or_else(|| {
    //     (StatusCode::CREATED, Json(opt)).into_response()
    // })
    println!("{:?}", payload);

    (StatusCode::CREATED, Json(payload)).into_response()
}

/// Cancel flight
///
/// Tell svc-scheduler to cancel a flight
#[utoipa::path(
    delete,
    path = "/cargo/cancel",
    responses(
        (status = 200, description = "Flight cancelled successfully"),
        (status = 404, description = "FlightOption not found")
    ),
    request_body = FlightCancel,
    security(
        (), // <-- make optional authentication
        ("api_key" = [])
    )
)]
pub async fn cancel_flight(Json(payload): Json<FlightCancel>, _headers: HeaderMap) -> StatusCode {
    // TODO Only allow specific user to cancel
    // TODO Query svc-auth

    // TODO svc-scheduler.cancel_flight
    println!("{:?}", payload);

    StatusCode::OK
}
