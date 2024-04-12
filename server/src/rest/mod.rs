#[macro_use]
pub mod macros;
pub mod api;
pub mod server;

use crate::rest_types;
use svc_scheduler_client_grpc::prelude::scheduler_storage::GeoPoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        api::request::request_flight,
        api::query::query_vertiports,
        api::confirm::confirm_itinerary,
        api::cancel::cancel_itinerary,
        api::scan::scan_parcel,
        api::query::query_landings,
        api::query::query_scans,
        api::health::health_check
    ),
    components(
        schemas(
            rest_types::Itinerary,
            rest_types::FlightLeg,
            rest_types::Vertiport,
            rest_types::ConfirmStatus,
            rest_types::VertiportsQuery,
            rest_types::ItineraryCancel,
            rest_types::FlightRequest,
            rest_types::ItineraryConfirm,
            rest_types::ItineraryConfirmation,
            rest_types::ParcelScan,
            rest_types::TimeWindow,
            rest_types::Landing,
            rest_types::LandingsQuery,
            rest_types::LandingsResponse,
            rest_types::TrackingQuery,
            rest_types::TrackingResponse,
            GeoPoint
        )
    ),
    tags(
        (name = "svc-cargo", description = "svc-cargo REST API")
    )
)]
struct ApiDoc;

/// Create OpenAPI3 Specification File
pub fn generate_openapi_spec(target: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = ApiDoc::openapi()
        .to_pretty_json()
        .expect("(ERROR) unable to write openapi specification to json.");

    std::fs::write(target, output).expect("(ERROR) unable to write json string to file.");

    Ok(())
}
