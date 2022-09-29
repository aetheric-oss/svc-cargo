//! Public API for the svc-cargo server
//! REST Endpoints for client applications

// use std::{
//     sync::Arc,
//     // collections::HashMap
// };

use chrono::naive::NaiveDateTime;

use axum::{extract::Query, response::IntoResponse, Json};
use hyper::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// Flight Query
#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone, IntoParams, ToSchema)]
pub struct FlightQuery {
    vport_depart_id: String,
    vport_arrive_id: String,
    timestamp: NaiveDateTime,
    weight_kg: f32,
}

impl FlightQuery {
    /// Creates a new flight query with required fields
    /// # Arguments
    /// vport_depart_id: The String ID of the vertiport to leave from
    /// vport_arrive_id: The String ID of the destination vertiport
    /// timestamp: The delivery time
    /// weight_kg: The approximate weight of the cargo
    pub fn new(
        vport_depart_id: String,
        vport_arrive_id: String,
        timestamp: NaiveDateTime,
        weight_kg: f32,
    ) -> Self {
        FlightQuery {
            vport_depart_id,
            vport_arrive_id,
            timestamp,
            weight_kg,
        }
    }
}

/// Region Query
#[derive(Debug, Deserialize, Copy, Clone, IntoParams, ToSchema)]
#[allow(dead_code)]
pub struct RegionQuery {
    latitude: f32,
    longitude: f32,
    // TODO Filters
}

impl RegionQuery {
    /// Creates a region query with required fields
    /// # Arguments
    /// lat: Latitude in Float format
    /// long: Longitude in Float format
    pub fn new(lat: f32, long: f32) -> Self {
        RegionQuery {
            latitude: lat,
            longitude: long,
        }
    }
}

/// Flight Plan Option
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct FlightOption {
    fp_id: String,
    vport_depart: String,
    vport_arrive: String,
    timestamp: NaiveDateTime,
}

/// Vertiport Information
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Vertiport {
    id: String,
    #[schema(example = "Mercy Hospital (Public)")]
    label: String,
    latitude: f32,
    longitude: f32,
}

// #[derive(Serialize, Deserialize, ToSchema, Clone)]
// pub struct VertiportInstructions {
//     id: String,
//     #[schema(example = "Check-In at Arrow Office, Floor 10 of West Tower")]
//     description_depart: String,
//     #[schema(example = "To Hamilton Street: Elevator to floor 2, take the pedestrian bridge to the street.")]
//     description_arrive: HashMap<String, String>
// }

/// Confirm Flight Operation Errors
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub enum ConfirmError {
    /// FlightOption already exists conflict.
    #[schema(example = "Could not confirm flight.")]
    Conflict(String),
    /// FlightOption not found by id.
    #[schema(example = "Provided flight plan ID doesn't match an existing flight.")]
    NotFound(String),
    /// Unauthorized Attempt to Confirm Flight
    #[schema(example = "Unauthorized confirmation by someone other than the customer.")]
    Unauthorized(String),
}

/// Get All Vertiports in a Region
///
/// List all Vertiport items from svc-storage
#[utoipa::path(
    get,
    path = "/region",
    params(
        RegionQuery
    ),
    responses(
        (status = 200, description = "List all vertiports successfully", body = [Vertiport])
    )
)]
pub async fn query_vertiports() -> Json<Vec<Vertiport>> {
    // let vertiports = store.lock().await.clone();
    // TODO Query svc-storage

    Json(vec![])
}

/// Search FlightOptions by query params.
///
/// Search `FlightOption`s by query params and return matching `FlightOption`s.
#[utoipa::path(
    get,
    path = "/flight",
    params(
        FlightQuery
    ),
    responses(
        (status = 200, description = "List possible berths", body = [FlightOption])
    )
)]
pub async fn query_flight(_query: Query<FlightQuery>) -> Json<Vec<FlightOption>> {
    // TODO get from svc-storage
    Json(vec![])
}

/// Confirm a Flight
///
/// Tries to confirm a flight with the svc-scheduler
#[utoipa::path(
    put,
    path = "/flight",
    request_body = String,
    responses(
        (status = 201, description = "Flight Confirmed", body = String),
        (status = 409, description = "Flight Confirmation Failed", body = ConfirmError)
    )
)]
pub async fn confirm_flight(Json(fp_id): Json<String>) -> impl IntoResponse {
    // TODO Confirm with svc-scheduler
    // Get Result
    // .unwrap_or_else(|| {
    //     (StatusCode::CREATED, Json(opt)).into_response()
    // })

    (StatusCode::CREATED, Json(fp_id)).into_response()
}

/// Cancel flight
///
/// Tell svc-scheduler to cancel a flight
#[utoipa::path(
    delete,
    path = "/flight",
    responses(
        (status = 200, description = "Flight cancelled successfully"),
        (status = 404, description = "FlightOption not found")
    ),
    request_body = String,
    security(
        (), // <-- make optional authentication
        ("api_key" = [])
    )
)]
pub async fn cancel_flight(Json(_fp_id): Json<String>, _headers: HeaderMap) -> StatusCode {
    // TODO Only allow specific user to cancel
    // TODO Query svc-auth

    // TODO svc-scheduler.cancel_flight

    StatusCode::OK
}
