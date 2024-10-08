use crate::grpc::client::GrpcClients;
use geo::HaversineDistance;
use hyper::StatusCode;
use svc_scheduler_client_grpc::prelude::scheduler_storage::GeoPointZ;
use svc_storage_client_grpc::prelude::*;
use svc_storage_client_grpc::resources::vehicle::Data as VehicleData;
use svc_storage_client_grpc::resources::vertipad::Data as VertipadData;
use svc_storage_client_grpc::resources::Id as StorageId;

/// Request a vertipad record by id
pub async fn get_vertipad_data(
    vertipad_id: &str,
    grpc_clients: &GrpcClients,
) -> Result<VertipadData, StatusCode> {
    grpc_clients
        .storage
        .vertipad
        .get_by_id(StorageId {
            id: vertipad_id.to_string(),
        })
        .await
        .map_err(|e| {
            rest_error!("could not get ID {vertipad_id} from svc-storage: {e}");
            StatusCode::NOT_FOUND
        })?
        .into_inner()
        .data
        .ok_or_else(|| {
            rest_error!("svc-storage response missing data.");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Get the vehicle's data from the storage service
pub async fn get_vehicle_data(
    vehicle_id: &str,
    grpc_clients: &GrpcClients,
) -> Result<VehicleData, StatusCode> {
    grpc_clients
        .storage
        .vehicle
        .get_by_id(Id {
            id: vehicle_id.to_string(),
        })
        .await
        .map_err(|e| {
            rest_error!("could not get ID {vehicle_id} from svc-storage: {e}");
            StatusCode::NOT_FOUND
        })?
        .into_inner()
        .data
        .ok_or_else(|| {
            rest_error!("svc-storage response missing data.");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Get the parent vertiport ID from one of its vertipad IDs
///  Necessary if the flight plan specifies vertipads and not vertiports
pub async fn get_vertiport_id_from_vertipad_id(
    grpc_clients: &GrpcClients,
    vertipad_id: &str,
) -> Result<String, StatusCode> {
    let vertiport_id = grpc_clients
        .storage
        .vertipad
        .get_by_id(StorageId {
            id: vertipad_id.to_string(),
        })
        .await
        .map_err(|e| {
            let error_msg = "svc-storage error searching vertipad.".to_string();
            rest_error!("{} {:?}", &error_msg, e);
            StatusCode::NOT_FOUND
        })?
        .into_inner()
        .data
        .ok_or_else(|| {
            rest_error!("vertipad data not found.");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .vertiport_id;

    Ok(vertiport_id)
}

/// Gets the total distance of a path in meters
/// TODO(R5): Temporary function to convert path to distance, until svc-storage is updated with it
pub fn get_distance_meters(path: &[GeoPointZ]) -> Option<f64> {
    // let mut distance: f64 = 0.0;
    if path.len() < 2 {
        rest_error!("path too short: {} segment(s).", path.len());

        return None;
    }

    let distance: f64 = path
        .windows(2)
        .map(|pair| {
            geo::point!(
                x: pair[0].x,
                y: pair[0].y
            )
            .haversine_distance(&geo::point!(
                x: pair[1].x,
                y: pair[1].y
            ))
        })
        .sum();

    Some(distance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_common::uuid::Uuid;

    // A rough conversion of the distance in meters for a degree of latitude
    fn degrees_to_latitude(degrees: f64) -> f64 {
        degrees * 111_111.0
    }

    // A rough conversion of distance in meters per degree of longitude
    //  The latitude affects this significantly
    fn degrees_to_longitude(degrees: f64, latitude: f64) -> f64 {
        degrees * 111_111.0 * latitude.to_radians().cos()
    }

    #[test]
    fn test_get_distance_meters() {
        let base = GeoPointZ {
            x: 5.167,
            y: 52.64,
            z: 0.0,
        };

        // path too short
        let path = vec![base];
        assert_eq!(get_distance_meters(&path), None);

        let target = GeoPointZ {
            x: base.x,
            y: base.y + 0.01,
            z: base.z,
        };

        let path = vec![base, target];

        let expected_distance_m = degrees_to_latitude((target.y - base.y).abs());
        let distance_m = get_distance_meters(&path).unwrap();

        // difference less than 5m
        let delta = (expected_distance_m - distance_m).abs();
        assert!(delta < 5.0);

        //
        // Longitude Difference
        //
        let target = GeoPointZ {
            x: base.x + 0.01,
            y: base.y,
            z: base.z,
        };

        let expected_distance_m = degrees_to_longitude((target.x - base.x).abs(), base.y);
        let path = vec![base, target];
        let distance_m = get_distance_meters(&path).unwrap();
        let delta = (expected_distance_m - distance_m).abs();

        ut_info!(
            "expected_distance_m: {}, distance_m: {}, delta: {}",
            expected_distance_m,
            distance_m,
            delta
        );
        assert!(delta < 5.0);
    }

    #[tokio::test]
    async fn test_get_vertiport_id_from_vertipad_id() {
        use svc_storage_client_grpc::resources::vertipad::Data as VertipadData;

        let vertipad_id = Uuid::new_v4().to_string();
        let expected_vertiport_id = Uuid::new_v4().to_string();

        let config = crate::config::Config::default();
        let grpc_clients = GrpcClients::default(config);

        // try get without inserting first
        let error = get_vertiport_id_from_vertipad_id(&grpc_clients, &vertipad_id)
            .await
            .unwrap_err();

        assert_eq!(error, StatusCode::NOT_FOUND);

        let vertipad_id = grpc_clients
            .storage
            .vertipad
            .insert(VertipadData {
                vertiport_id: expected_vertiport_id.to_string(),
                ..Default::default()
            })
            .await
            .unwrap()
            .into_inner()
            .object
            .unwrap()
            .id;

        let vertiport_id = get_vertiport_id_from_vertipad_id(&grpc_clients, &vertipad_id)
            .await
            .unwrap();

        assert_eq!(expected_vertiport_id, vertiport_id);
    }

    #[tokio::test]
    async fn test_get_vehicle_data() {
        let config = crate::config::Config::default();
        let grpc_clients = GrpcClients::default(config);

        let vehicle_id = Uuid::new_v4().to_string();

        // try to get without insertion
        let error = get_vehicle_data(&vehicle_id, &grpc_clients)
            .await
            .unwrap_err();
        assert_eq!(error, StatusCode::NOT_FOUND);

        let vehicle_data = VehicleData::default();
        let vehicle_id = grpc_clients
            .storage
            .vehicle
            .insert(vehicle_data)
            .await
            .unwrap()
            .into_inner()
            .object
            .unwrap()
            .id;

        let _ = get_vehicle_data(&vehicle_id, &grpc_clients).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_vertipad_data() {
        let config = crate::config::Config::default();
        let grpc_clients = GrpcClients::default(config);

        let vertipad_id = Uuid::new_v4().to_string();

        // try to get without insertion
        let error = get_vertipad_data(&vertipad_id, &grpc_clients)
            .await
            .unwrap_err();
        assert_eq!(error, StatusCode::NOT_FOUND);

        let vertipad_data = VertipadData::default();
        let vertipad_id = grpc_clients
            .storage
            .vertipad
            .insert(vertipad_data)
            .await
            .unwrap()
            .into_inner()
            .object
            .unwrap()
            .id;

        let _ = get_vertipad_data(&vertipad_id, &grpc_clients)
            .await
            .unwrap();
    }
}
