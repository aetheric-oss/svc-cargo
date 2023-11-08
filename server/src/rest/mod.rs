#[macro_use]
pub mod macros;
pub mod server;

mod api;
use api::*;

use utoipa::OpenApi;

use svc_scheduler_client_grpc::prelude::scheduler_storage::GeoPoint;

#[derive(OpenApi)]
#[openapi(
    paths(
        request::request_flight,
        query::query_vertiports,
        confirm::confirm_itinerary,
        cancel::cancel_itinerary,
        scan::scan_parcel,
        query::query_landings,
        query::query_scans,
        health::health_check
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
