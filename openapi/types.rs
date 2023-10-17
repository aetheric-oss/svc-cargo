use chrono::{DateTime, Utc};
/// Types used for REST communication with the svc-cargo server
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use svc_scheduler_client_grpc::prelude::scheduler_storage::GeoPoint;

/// Don't allow overly large numbers of landings to be returned
pub const MAX_LANDINGS_TO_RETURN: u32 = 50;

/// Request Body Information for Flight Query
#[derive(Debug, Clone, IntoParams, ToSchema, Deserialize, Serialize)]
pub struct FlightRequest {
    /// The String ID of the vertiport to leave from
    pub vertiport_depart_id: String,

    /// The String ID of the destination vertiport
    pub vertiport_arrive_id: String,

    /// The window of departure
    pub time_depart_window: Option<TimeWindow>,

    /// The window of arrival
    pub time_arrive_window: Option<TimeWindow>,

    /// The estimated weight of cargo
    pub cargo_weight_kg: f32,
}

/// Time window (min and max)
#[derive(Debug, Copy, Clone, IntoParams, ToSchema, Deserialize, Serialize)]
pub struct TimeWindow {
    /// The start of the pad window
    pub timestamp_min: DateTime<Utc>,

    /// The end of the pad window
    pub timestamp_max: DateTime<Utc>,
}

/// Request body information to cancel an itinerary
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ItineraryCancel {
    /// Itinerary UUID to Cancel
    pub id: String,
}

/// Request Body Information for Region Query
#[derive(Debug, Copy, Clone, Deserialize, Serialize, ToSchema)]
pub struct VertiportsQuery {
    /// Latitude of Client
    pub latitude: f32,

    /// Longitude of Client
    pub longitude: f32,
}

/// Itinerary
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Itinerary {
    /// The UUID of the itinerary
    pub id: String,

    /// Each leg of the itinerary
    pub legs: Vec<FlightLeg>,

    /// The currency type, e.g. USD, EUR
    pub currency_type: Option<String>,

    /// The cost of the trip for the customer
    pub base_pricing: Option<f32>,
}

/// Leg of a flight
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct FlightLeg {
    /// Flight Plan ID
    pub flight_plan_id: String,

    /// Departure Vertiport ID
    pub vertiport_depart_id: String,

    /// Arrival Vertiport ID
    pub vertiport_arrive_id: String,

    /// Estimated departure timestamp
    pub timestamp_depart: DateTime<Utc>,

    /// Estimated arrival timestamp
    pub timestamp_arrive: DateTime<Utc>,

    /// The path of the flight plan
    pub path: Vec<GeoPoint>,

    /// The estimated trip distance in meters
    pub distance_meters: f32,

    /// The currency type, e.g. USD, EUR
    pub currency_type: Option<String>,

    /// The cost of the trip for the customer
    pub base_pricing: Option<f32>,
}

/// Customer Itinerary Confirm Option
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ItineraryConfirm {
    /// Itinerary UUID
    pub id: String,

    /// User ID
    pub user_id: String,

    /// Weight of Cargo
    /// TODO(R4): this is a little clunky to re-issue the weight here
    pub weight_grams: u32,
}

/// UUIDs of the confirmed flight
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ItineraryConfirmation {
    /// UUID of the itinerary
    pub itinerary_id: String,

    /// UUID of the package
    pub parcel_id: String,
}

/// Vertiport Information
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Vertiport {
    /// The unique ID of the vertiport
    pub id: String,

    /// The human-readable label of the vertiport
    #[schema(example = "Mercy Hospital (Public)")]
    pub label: String,

    /// The latitude (float value) of the vertiport (centroid)
    pub latitude: f32,

    /// The longitude (float value) of the vertiport (centroid)
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

/// Confirm itinerary Operation Status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub enum ConfirmStatus {
    /// Successful confirmation of itinerary
    #[schema(example = "Itinerary successfully confirmed.")]
    Success(String),

    /// Itinerary already confirmed.
    #[schema(example = "Could not confirm itinerary.")]
    Conflict(String),

    /// Itinerary not found by id.
    #[schema(example = "Provided itinerary ID doesn't match an existing itinerary.")]
    NotFound(String),

    /// Unauthorized Attempt to Confirm Itinerary
    #[schema(example = "Unauthorized confirmation by someone other than the customer.")]
    Unauthorized(String),

    /// Unavailable Service
    Unavailable,
}

/// Vertiport Information
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ParcelScan {
    /// The unique ID (UUID) of the scanner device
    pub scanner_id: String,

    /// The unique ID (UUID) of the parcel
    pub parcel_id: String,

    /// The latitude (float value) of the scan location
    pub latitude: f64,

    /// The longitude (float value) of the scan location
    pub longitude: f64,
}

/// Request Body Information for Landings at a given vertiport
#[derive(Debug, Clone, IntoParams, ToSchema, Deserialize, Serialize)]
pub struct LandingsQuery {
    /// The String ID of the vertiport
    pub vertiport_id: String,

    /// The window to search for landings
    pub arrival_window: Option<TimeWindow>,

    /// The maximum number of landings to return (max: [`MAX_LANDINGS_TO_RETURN`]])
    pub limit: u32,
}

/// Landings Response
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct LandingsResponse {
    /// list of landing information
    pub landings: Vec<Landing>,
}

/// Landing
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Landing {
    /// The String ID of the flight plan
    pub flight_plan_id: String,

    /// Vertipad Name
    pub vertipad_name: String,

    /// The callsign of the aircraft
    pub aircraft_callsign: String,

    /// The time of arrival
    pub timestamp: DateTime<Utc>,
    // TODO(R4) Aircraft Nickname
    // pub aircraft_nickname: String,

    // TODO(R4) Estimated Dwell Time
    // pub estimated_dwell_seconds: u32,

    // TODO(R4) Parcels to deliver and acquire
    // pub parcels_deliver: Vec<String>,
    // pub parcels_acquire: Vec<String>,
}

/// Request Body Information for Tracking a Parcel Query
#[derive(Debug, Clone, IntoParams, ToSchema, Deserialize, Serialize)]
pub struct TrackingQuery {
    /// The String ID of the vertiport
    pub parcel_id: String,
}

/// Tracking Information Response
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct TrackingResponse {
    /// list of scans
    pub scans: Vec<ParcelScan>,
}
