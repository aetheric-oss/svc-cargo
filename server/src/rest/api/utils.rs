use crate::grpc::client::GrpcClients;
use hyper::StatusCode;
use svc_storage_client_grpc::prelude::*;
use svc_storage_client_grpc::resources::vehicle::Data as VehicleData;
use svc_storage_client_grpc::resources::vertipad::Data as VertipadData;
use uuid::Uuid;

/// Don't allow large UUID strings
const UUID_MAX_SIZE: usize = 50; // Sometimes braces or hyphens

/// Returns true if a given string is UUID format
pub fn is_uuid(s: &str) -> bool {
    // Prevent buffer overflows
    if s.len() > UUID_MAX_SIZE {
        rest_error!("(is_uuid) input string larger than expected: {}.", s.len());
        return false;
    }

    Uuid::parse_str(s).is_ok()
}

/// Request a vertipad record by id
pub async fn get_vertipad_details(
    vertipad_id: &str,
    grpc_clients: &GrpcClients,
) -> Result<VertipadData, StatusCode> {
    let request = Id {
        id: vertipad_id.to_string(),
    };

    let response = match grpc_clients.storage.vertipad.get_by_id(request).await {
        Ok(response) => response.into_inner(),
        Err(e) => {
            let error_msg = "svc-storage error.".to_string();
            rest_error!("(get_landings) {} {:?}", &error_msg, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let Some(data) = response.data else {
        let error_msg = "svc-storage error; no data.".to_string();
        rest_error!("(get_landings) {}", &error_msg);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    Ok(data)
}

pub async fn get_vehicle_details(
    vehicle_id: &str,
    grpc_clients: &GrpcClients,
) -> Result<VehicleData, StatusCode> {
    let request = Id {
        id: vehicle_id.to_string(),
    };

    let response = match grpc_clients.storage.vehicle.get_by_id(request).await {
        Ok(response) => response.into_inner(),
        Err(e) => {
            let error_msg = "svc-storage error, could not get by id.".to_string();
            rest_error!("(get_landings) {} {:?}", &error_msg, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let Some(data) = response.data else {
        let error_msg = "svc-storage error; no data.".to_string();
        rest_error!("(get_landings) {}", &error_msg);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    Ok(data)
}
