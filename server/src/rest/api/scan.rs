use super::utils::is_uuid;
use crate::grpc::client::GrpcClients;
use crate::rest_types::ParcelScan;
use axum::{extract::Extension, Json};
use hyper::StatusCode;
use svc_storage_client_grpc::resources::parcel_scan::Data as ParcelScanData;
use svc_storage_client_grpc::ClientConnect;
use svc_storage_client_grpc::GeoPoint;

/// Scan a parcel
/// The provided parcel ID and scanner ID must already exist in the database
#[utoipa::path(
    put,
    path = "/cargo/scan",
    tag = "svc-cargo",
    request_body = ParcelScan,
    responses(
        (status = 200, description = "Scan succeeded", body = String),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "svc-storage returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    )
)]
pub async fn scan_parcel(
    Extension(grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<ParcelScan>,
) -> Result<(), StatusCode> {
    rest_debug!("(scan_parcel) entry.");

    if !is_uuid(&payload.parcel_id) {
        let error_msg = "parcel ID not in UUID format.".to_string();
        rest_error!("(scan_parcel) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    if !is_uuid(&payload.scanner_id) {
        let error_msg = "scanner ID not in UUID format.".to_string();
        rest_error!("(scan_parcel) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

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

    // Get Client
    let Ok(mut client) = grpc_clients.storage.parcel_scan.get_client().await else {
        let error_msg = "svc-storage unavailable.".to_string();
        rest_error!("(scan_parcel) {}", &error_msg);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    // Make request, process response
    let request = tonic::Request::new(ParcelScanData {
        scanner_id: payload.scanner_id,
        parcel_id: payload.parcel_id,
        geo_location: Some(GeoPoint {
            latitude: payload.longitude,
            longitude: payload.latitude,
        }),
    });

    let response = match client.insert(request).await {
        Ok(response) => response.into_inner(),
        Err(e) => {
            let error_msg = "svc-storage error.".to_string();
            rest_error!("(scan_parcel) {} {:?}", &error_msg, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let Some(response) = response.validation_result else {
        let error_msg = "svc-storage response invalid.".to_string();
        rest_error!("(scan_parcel) {}", &error_msg);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    if response.success {
        rest_info!("(scan_parcel) svc-storage success.");
        Ok(())
    } else {
        let error_msg = "svc-storage failure.".to_string();
        rest_error!("(scan_parcel) {}", &error_msg);
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}
