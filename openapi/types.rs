use chrono::{DateTime, Utc};
/// Types used for REST communication with the svc-cargo server
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
pub use svc_scheduler_client_grpc::prelude::scheduler_storage::{GeoPoint, flight_plan::Data as FlightPlan};
pub use svc_scheduler_client_grpc::client::TaskResponse;

/// Don't allow overly large numbers of occupations to be returned
pub const MAX_LANDINGS_TO_RETURN: u32 = 50;

/// Request Body Information for Flight Query
#[derive(Debug, Clone, IntoParams, ToSchema, Deserialize, Serialize)]
pub struct QueryItineraryRequest {
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
pub struct ItineraryCancelRequest {
    /// Itinerary UUID to Cancel
    pub id: String,
}

/// Request Body Information for Region Query
#[derive(Debug, Copy, Clone, Deserialize, Serialize, ToSchema)]
pub struct QueryVertiportsRequest {
    /// Latitude of Client
    pub latitude: f32,

    /// Longitude of Client
    pub longitude: f32,
}

/// Supported Currencies
#[derive(Debug, Serialize, Deserialize, Copy, Clone, ToSchema)]
pub enum CurrencyUnit {
    /// One U.S. Dollar
    USD,

    /// One E.U. Euro
    EURO,
}

/// Itinerary
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Itinerary {
    /// The svc-cargo UUID of the itinerary
    /// Use to confirm a possible itinerary
    pub id: String,

    /// Each leg of the itinerary
    pub flight_plans: Vec<FlightPlan>,

    /// The currency type, e.g. USD, EUR
    pub currency_units: CurrencyUnit,

    /// The cost of the trip for the customer
    /// List of "item": "cost"
    pub base_pricing: Vec<(String, f32)>,
}

// /// Leg of a flight
// #[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
// pub struct FlightPlan {
//     /// Flight Plan ID
//     pub flight_plan_id: String,

//     /// Departure Vertiport ID
//     pub vertiport_depart_id: String,

//     /// Arrival Vertiport ID
//     pub vertiport_arrive_id: String,

//     /// Estimated departure timestamp
//     pub timestamp_depart: DateTime<Utc>,

//     /// Estimated arrival timestamp
//     pub timestamp_arrive: DateTime<Utc>,

//     /// The path of the flight plan
//     pub path: Vec<GeoPoint>,

//     /// The estimated trip distance in meters
//     pub distance_meters: f32,

//     /// The currency type, e.g. USD, EUR
//     pub currency_type: Option<String>,

//     /// The cost of the trip for the customer
//     pub base_pricing: Option<f32>,
// }

/// Customer Itinerary Confirm Option
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ItineraryCreateRequest {
    /// The svc-cargo itinerary ID to create
    pub id: String
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
pub enum ItineraryCreateStatus {
    /// Successful creation of itinerary
    #[schema(example = "Itinerary successfully created.")]
    Success(String),

    /// Itinerary already created.
    #[schema(example = "Could not create itinerary.")]
    Conflict(String),

    /// Itinerary not found by id.
    #[schema(example = "Provided itinerary ID doesn't match an existing itinerary.")]
    NotFound(String),

    /// Unauthorized Attempt to Confirm Itinerary
    #[schema(example = "Unauthorized creation by someone other than the customer.")]
    Unauthorized(String),

    /// Unavailable Service
    Unavailable,
}

/// Request Body Information for Occupations at a Given Vertiport
#[derive(Debug, Clone, IntoParams, ToSchema, Deserialize, Serialize)]
pub struct QueryScheduleRequest {
    /// The String ID of the vertiport
    pub vertiport_id: String,

    /// The window to search for occupations
    pub arrival_window: Option<TimeWindow>,

    /// The maximum number of occupations to return (max: [`MAX_LANDINGS_TO_RETURN`]])
    pub limit: u32,
}

/// Occupations Response
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct QueryScheduleResponse {
    /// list of landing information
    pub occupations: Vec<Occupation>,
}

/// Information about a parcel
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ParcelInfo {
    /// the unique UUID of the parcel
    pub parcel_id: String,

    /// the nickname of the parcel
    pub parcel_nickname: Option<String>

    // TODO(R5): weight, etc.
}

/// Vertipad Occupation
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Occupation {
    /// The unique UUID of the flight plan
    pub flight_plan_id: String,

    /// Unique vertipad UUID
    pub vertipad_id: String,

    /// The human-readable label of the vertipad
    pub vertipad_display_name: Option<String>,

    /// The time window of occupation
    pub time_window: TimeWindow,

    /// The callsign of the aircraft
    pub aircraft_callsign: String,

    /// The nickname of the aircraft
    pub aircraft_nickname: Option<String>,

    /// Parcels being picked up during this occupation
    pub parcels_acquire: Vec<ParcelInfo>,

    /// Parcels being delivered during this occupation
    pub parcels_deliver: Vec<ParcelInfo>,
}

/// Request Body Information for Tracking a Parcel
#[derive(Debug, Clone, IntoParams, ToSchema, Deserialize, Serialize)]
pub struct QueryParcelRequest {
    /// The UUID of the parcel
    pub parcel_id: String,
}

/// ParcelScan information
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

    /// The timestamp of the scan
    pub timestamp: DateTime<Utc>
}

/// Tracking Information Response
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct QueryParcelResponse {
    /// list of scans
    pub scans: Vec<ParcelScan>,
}
