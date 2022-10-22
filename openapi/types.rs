/// Types used for REST communication with the svc-cargo server

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use chrono::NaiveDateTime;

/// API Endpoint to Cancel a Flight
pub const ENDPOINT_CANCEL: &str = "/cargo/cancel";

/// API Endpoint to Confirm a Flight
pub const ENDPOINT_CONFIRM: &str = "/cargo/confirm";

/// API Endpoint to Query for Available Flights
pub const ENDPOINT_QUERY: &str = "/cargo/query";

/// API Endpoint to Get Vertiports for a Region
pub const ENDPOINT_VERTIPORTS: &str = "/cargo/vertiports";

/// Request Body Information for Flight Query
#[allow(dead_code)]
#[derive(Debug, Clone, IntoParams, ToSchema)]
#[derive(Deserialize, Serialize)]
pub struct FlightQuery {
    vertiport_depart_id: String,
    vertiport_arrive_id: String,
    timestamp_depart_min: NaiveDateTime,
    timestamp_depart_max: NaiveDateTime,
    cargo_weight_kg: f32,
}

impl FlightQuery {
    /// Creates a new flight query with required fields
    /// # Arguments
    /// vertiport_depart_id: The String ID of the vertiport to leave from
    /// vertiport_arrive_id: The String ID of the destination vertiport
    /// timestamp_depart_min: The start of the pad departure window
    /// timestamp_depart_max: The end of the pad departure window
    /// cargo_weight_kg: The approximate weight of the cargo
    #[allow(dead_code)]
    pub fn new(
        vertiport_depart_id: String,
        vertiport_arrive_id: String,
        timestamp_depart_min: NaiveDateTime,
        timestamp_depart_max: NaiveDateTime,
        cargo_weight_kg: f32,
    ) -> Self {
        FlightQuery {
            vertiport_depart_id,
            vertiport_arrive_id,
            timestamp_depart_min,
            timestamp_depart_max,
            cargo_weight_kg,
        }
    }
}

/// Request Body Information to Cancel a Flight
#[derive(Debug, Clone)]
#[derive(Deserialize, Serialize)]
#[derive(ToSchema)]
pub struct FlightCancel {

    /// Flight Plan ID to Cancel
    pub fp_id: String,
    // TODO optional reason
}

/// Request Body Information for Region Query
#[derive(Debug, Copy, Clone)]
#[derive(Deserialize, Serialize)]
#[derive(ToSchema)]
pub struct VertiportsQuery {
    latitude: f32,
    longitude: f32,
}

impl VertiportsQuery {
    /// Creates a region query with required fields
    /// # Arguments
    /// lat: Latitude in Float format
    /// long: Longitude in Float format
    #[allow(dead_code)]
    pub fn new(lat: f32, long: f32) -> Self {
        VertiportsQuery {
            latitude: lat,
            longitude: long,
        }
    }
}

/// Flight Plan Option
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct FlightOption {
    /// Flight Plan ID
    pub fp_id: String,

    /// Departure Vertiport ID
    pub vertiport_depart_id: String,

    /// Arrival Vertiport ID
    pub vertiport_arrive_id: String,

    /// Estimated departure timestamp
    pub timestamp_depart: NaiveDateTime,

    /// Estimated arrival timestamp
    pub timestamp_arrive: NaiveDateTime
}


/// Customer Flight Confirm Option
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[derive(ToSchema)]
pub struct FlightConfirm {

    /// Flight Plan ID
    pub fp_id: String
}
/// Vertiport Information
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Vertiport {
    /// The unique ID of the vertiport
    pub id: u32,

    /// The human-readable label of the vertiport
    #[schema(example = "Mercy Hospital (Public)")]
    pub label: String,

    /// The latitude (float value) of the vertiport
    pub latitude: f32,

    /// The longitude (float value) of the vertiport
    pub longitude: f32,
}

// #[derive(Serialize, Deserialize, ToSchema, Clone)]
// pub struct VertiportInstructions {
//     id: String,
//     #[schema(example = "Check-In at Arrow Office, Floor 10 of West Tower")]
//     description_depart: String,
//     #[schema(example = "To Hamilton Street: Elevator to floor 2, take the pedestrian bridge to the street.")]
//     description_arrive: HashMap<String, String>
// }

/// Confirm Flight Operation Status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub enum ConfirmStatus {
    /// Successful confirmation of flight
    #[schema(example = "Flight successfully confirmed.")]
    Success(String),

    /// FlightOption already exists conflict.
    #[schema(example = "Could not confirm flight.")]
    Conflict(String),
    /// FlightOption not found by id.
    #[schema(example = "Provided flight plan ID doesn't match an existing flight.")]
    NotFound(String),
    /// Unauthorized Attempt to Confirm Flight
    #[schema(example = "Unauthorized confirmation by someone other than the customer.")]
    Unauthorized(String),

    /// Unavailable Service
    Unavailable,
}
