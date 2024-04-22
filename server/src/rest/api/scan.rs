use super::rest_types::CargoScan;
use crate::grpc::client::GrpcClients;
use axum::{extract::Extension, Json};
use hyper::StatusCode;
use lib_common::time::Utc;
use lib_common::uuid::to_uuid;
use svc_storage_client_grpc::prelude::*;
use svc_storage_client_grpc::resources::parcel_scan::Data as CargoScanData;

/// Scan a parcel
/// The provided parcel ID and scanner ID must already exist in the database
#[utoipa::path(
    put,
    path = "/cargo/scan",
    tag = "svc-cargo",
    request_body = CargoScan,
    responses(
        (status = 200, description = "Scan succeeded", body = String),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "svc-storage returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    )
)]
pub async fn scan_parcel(
    Extension(grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<CargoScan>,
) -> Result<(), StatusCode> {
    rest_debug!("(scan_parcel) entry.");

    // TODO(R5): Consider too old timestamps?
    //  Maybe an offline scanner could store scans until it has a connection

    to_uuid(&payload.parcel_id).ok_or_else(|| {
        rest_error!("(scan_parcel) parcel ID not in UUID format.");
        StatusCode::BAD_REQUEST
    })?;

    to_uuid(&payload.scanner_id).ok_or_else(|| {
        rest_error!("(scan_parcel) scanner ID not in UUID format.");
        StatusCode::BAD_REQUEST
    })?;

    if payload.latitude < -90.0
        || payload.latitude > 90.0
        || payload.longitude < -180.0
        || payload.longitude > 180.0
    {
        let error_msg = "coordinates out of range.".to_string();
        rest_error!(
            "(scan_parcel) {}: (lat: {}, lon: {})",
            &error_msg,
            payload.latitude,
            payload.longitude
        );

        return Err(StatusCode::BAD_REQUEST);
    }

    // Make request, process response
    let data = CargoScanData {
        scanner_id: payload.scanner_id,
        parcel_id: payload.parcel_id,
        geo_location: Some(GeoPoint {
            latitude: payload.longitude,
            longitude: payload.latitude,
            altitude: payload.altitude,
        }),
        created_at: Some(Utc::now().into()),
    };

    #[cfg(not(tarpaulin_include))]
    // no_coverage: need backends to test (integration)
    grpc_clients
        .storage
        .parcel_scan
        .insert(data)
        .await
        .map_err(|e| {
            rest_error!("(scan_parcel) svc-storage error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_inner()
        .validation_result
        .ok_or_else(|| {
            rest_error!("(scan_parcel) svc-storage response missing validation result.");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .success
        .then(|| {
            rest_info!("(scan_parcel) svc-storage success.");
        })
        .ok_or_else(|| {
            rest_error!("(scan_parcel) svc-storage failure.");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scan_parcel_nominal() {
        let config = crate::config::Config::default();
        let grpc_clients = GrpcClients::default(config);
        let parcel_id = "00000000-0000-0000-0000-000000000000";
        let scanner_id = "00000000-0000-0000-0000-000000000001";
        let latitude = 0.0;
        let longitude = 0.0;
        let altitude = 0.0;
        let timestamp = Utc::now().into();

        scan_parcel(
            Extension(grpc_clients),
            Json(CargoScan {
                parcel_id: parcel_id.to_string(),
                scanner_id: scanner_id.to_string(),
                latitude,
                longitude,
                altitude,
                timestamp,
            }),
        )
        .await
        .unwrap(); // should succeed
    }

    #[tokio::test]
    async fn test_scan_parcel_invalid_ids() {
        let config = crate::config::Config::default();
        let grpc_clients = GrpcClients::default(config);
        let parcel_id = "00000000-0000-0000-0000-000000000000";
        let scanner_id = "00000000-0000-0000-0000-000000000001";

        let mut scan_data = CargoScan {
            parcel_id: parcel_id.to_string().replace("-", ""),
            scanner_id: scanner_id.to_string(),
            latitude: 90.0,
            longitude: 180.0,
            altitude: 0.0,
            timestamp: Utc::now().into(),
        };

        let result = scan_parcel(Extension(grpc_clients.clone()), Json(scan_data.clone()))
            .await
            .unwrap_err();
        assert_eq!(result, StatusCode::BAD_REQUEST);
        scan_data.parcel_id = parcel_id.to_string();

        // Bad scanner ID
        scan_data.scanner_id = scanner_id.to_string().replace("-", "");
        let result = scan_parcel(Extension(grpc_clients.clone()), Json(scan_data.clone()))
            .await
            .unwrap_err();
        assert_eq!(result, StatusCode::BAD_REQUEST);
        scan_data.scanner_id = scanner_id.to_string();

        // reset
        scan_parcel(Extension(grpc_clients.clone()), Json(scan_data.clone()))
            .await
            .unwrap(); // should succeed

        // bad latitude > 90
        for latitude in [-90.01, 90.01] {
            scan_data.latitude = latitude;
            let result = scan_parcel(Extension(grpc_clients.clone()), Json(scan_data.clone()))
                .await
                .unwrap_err();
            assert_eq!(result, StatusCode::BAD_REQUEST);
        }
        scan_data.latitude = 0.0;

        // bad longitude
        for longitude in [-180.01, 180.01] {
            scan_data.longitude = longitude;
            let result = scan_parcel(Extension(grpc_clients.clone()), Json(scan_data.clone()))
                .await
                .unwrap_err();
            assert_eq!(result, StatusCode::BAD_REQUEST);
        }
        scan_data.longitude = 0.0;
    }
}
