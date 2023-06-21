#[macro_use]
pub mod macros;
pub mod api;
pub mod server;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        api::query_flight,
        api::query_vertiports,
        api::confirm_itinerary,
        api::cancel_itinerary,
        api::scan_parcel,
        api::query_landings
    ),
    components(
        schemas(
            api::rest_types::Itinerary,
            api::rest_types::FlightLeg,
            api::rest_types::Vertiport,
            api::rest_types::ConfirmStatus,
            api::rest_types::VertiportsQuery,
            api::rest_types::ItineraryCancel,
            api::rest_types::FlightQuery,
            api::rest_types::ItineraryConfirm,
            api::rest_types::ItineraryConfirmation,
            api::rest_types::ParcelScan,
            api::rest_types::TimeWindow,
            api::rest_types::Landing,
            api::rest_types::LandingsQuery,
            api::rest_types::LandingsResponse
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
