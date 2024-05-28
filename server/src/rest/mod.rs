#[macro_use]
pub mod macros;
pub mod server;

pub(crate) mod api;
use api::*;

use std::fmt::{self, Display, Formatter};
use utoipa::OpenApi;

/// OpenAPI 3.0 specification for this service
#[derive(OpenApi, Copy, Clone, Debug)]
#[openapi(
    paths(
        request::request_flight,
        query::query_vertiports,
        create::create_itinerary,
        cancel::cancel_itinerary,
        scan::scan_parcel,
        query::query_occupations,
        query::query_scans,
        health::health_check
    ),
    components(
        schemas(
            rest_types::Itinerary,
            rest_types::FlightPlan,
            rest_types::Vertiport,
            rest_types::QueryVertiportsRequest,
            rest_types::ItineraryCancelRequest,
            rest_types::QueryItineraryRequest,
            rest_types::DraftItinerary,
            rest_types::ItineraryCreateRequest,
            rest_types::CargoScan,
            rest_types::TimeWindow,
            rest_types::Occupation,
            rest_types::CargoInfo,
            rest_types::CurrencyUnit,
            rest_types::QueryScheduleRequest,
            rest_types::QueryScheduleResponse,
            rest_types::QueryParcelResponse,
            rest_types::GeoPoint,
            rest_types::PaymentInfo,
            rest_types::InvoiceItem
        )
    ),
    tags(
        (name = "svc-cargo", description = "svc-cargo REST API")
    )
)]
pub struct ApiDoc;

/// Errors with OpenAPI generation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpenApiError {
    /// Failed to export as JSON string
    Json,

    /// Failed to write to file
    FileWrite,
}

impl std::error::Error for OpenApiError {}

impl Display for OpenApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            OpenApiError::Json => write!(f, "Failed to export as JSON string"),
            OpenApiError::FileWrite => write!(f, "Failed to write to file"),
        }
    }
}

/// Create OpenAPI 3.0 Specification File
pub fn generate_openapi_spec<T>(target: &str) -> Result<(), OpenApiError>
where
    T: OpenApi,
{
    #[cfg(not(tarpaulin_include))]
    // no_coverage: no way to make JSON export fail
    let output = T::openapi().to_pretty_json().map_err(|e| {
        rest_error!("failed to export as JSON string: {e}");
        OpenApiError::Json
    })?;

    std::fs::write(target, output).map_err(|e| {
        rest_error!("failed to write to file: {e}");
        OpenApiError::FileWrite
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_openapi_spec() {
        let target = "/nonsense/";
        let error = generate_openapi_spec::<ApiDoc>(target).unwrap_err();
        assert_eq!(error, OpenApiError::FileWrite);

        // TODO(R5): Is it possible to make the JSON export fail?
        // #[derive(OpenApi)]
        // #[openapi(
        //     paths(invalid)
        // )]
        // struct InvalidApi;
        // let error = generate_openapi_spec::<InvalidApi>("test.json").unwrap_err();
        // assert_eq!(error, OpenApiError::Json);
    }

    #[test]
    fn test_openapi_error_display() {
        assert_eq!(
            format!("{}", OpenApiError::Json),
            "Failed to export as JSON string"
        );
        assert_eq!(
            format!("{}", OpenApiError::FileWrite),
            "Failed to write to file"
        );
    }
}
