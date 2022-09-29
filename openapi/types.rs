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
pub const ENDPOINT_REGION: &str = "/cargo/region";

/// Request Body Information for Flight Query
#[allow(dead_code)]
#[derive(Debug, Clone, IntoParams, ToSchema)]
#[derive(Deserialize, Serialize)]
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
    #[allow(dead_code)]
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
pub struct RegionQuery {
    latitude: f32,
    longitude: f32,
}

impl RegionQuery {
    /// Creates a region query with required fields
    /// # Arguments
    /// lat: Latitude in Float format
    /// long: Longitude in Float format
    #[allow(dead_code)]
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
