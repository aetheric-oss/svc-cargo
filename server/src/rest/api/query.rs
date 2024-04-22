use super::rest_types::{
    CargoScan, Occupation, QueryParcelResponse, QueryScheduleRequest, QueryScheduleResponse,
    QueryVertiportsRequest, TimeWindow, Vertiport, MAX_LANDINGS_TO_RETURN,
};
use crate::grpc::client::GrpcClients;
use axum::{extract::Path, Extension, Json};
use hyper::StatusCode;
use lib_common::uuid::{to_uuid, Uuid};
use std::fmt::{self, Display, Formatter};
use svc_storage_client_grpc::prelude::{
    flight_plan, parcel_scan, vertiport, AdvancedSearchFilter, SortOption, SortOrder,
};
use svc_storage_client_grpc::simple_service::Client;

#[derive(Debug, PartialEq)]
pub enum ScanError {
    Data,
    Location,
    CreatedAt,
}

impl TryFrom<parcel_scan::Object> for CargoScan {
    type Error = ScanError;

    fn try_from(obj: parcel_scan::Object) -> Result<Self, Self::Error> {
        let data = obj.data.ok_or_else(|| {
            rest_error!("(CargoScan) parcel scan data is None.");
            ScanError::Data
        })?;

        let geo_location = data.geo_location.ok_or_else(|| {
            rest_error!("(CargoScan) parcel scan location is None.");
            ScanError::Location
        })?;

        let created_at = data.created_at.ok_or_else(|| {
            rest_error!("(CargoScan) parcel scan created_at is None.");
            ScanError::CreatedAt
        })?;

        Ok(CargoScan {
            parcel_id: obj.id,
            scanner_id: data.scanner_id,
            latitude: geo_location.latitude,
            longitude: geo_location.longitude,
            altitude: geo_location.altitude,
            timestamp: created_at.into(),
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum VertiportError {
    Data,
    Location,
    Exterior,
}

impl TryFrom<vertiport::Object> for Vertiport {
    type Error = VertiportError;

    fn try_from(obj: vertiport::Object) -> Result<Self, Self::Error> {
        let data = obj.data.ok_or_else(|| {
            rest_error!("(Vertiport) vertiport data is None.");
            VertiportError::Data
        })?;

        let location = data.geo_location.ok_or_else(|| {
            rest_error!("(Vertiport) vertiport location is None.");
            VertiportError::Location
        })?;

        let exterior = location.exterior.ok_or_else(|| {
            rest_error!("(Vertiport) vertiport exterior is None.");
            VertiportError::Exterior
        })?;

        let points = exterior.points;
        let latitude = points.iter().map(|pt| pt.latitude).sum::<f64>() / points.len() as f64;
        let longitude = points.iter().map(|pt| pt.longitude).sum::<f64>() / points.len() as f64;

        Ok(Vertiport {
            id: obj.id,
            label: data.name,
            latitude: latitude as f32,
            longitude: longitude as f32,
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum OccupationError {
    Data,
    TargetTimeslotStart,
    TargetTimeslotEnd,
}

impl TryFrom<flight_plan::Object> for Occupation {
    type Error = OccupationError;

    fn try_from(obj: flight_plan::Object) -> Result<Self, Self::Error> {
        let data = obj.data.ok_or_else(|| {
            rest_error!("(Occupation) flight plan data is None.");
            OccupationError::Data
        })?;

        let target_timeslot_start = data.target_timeslot_start.ok_or_else(|| {
            rest_error!("(Occupation) flight plan target_timeslot_start is None.");
            OccupationError::TargetTimeslotStart
        })?;

        let target_timeslot_end = data.target_timeslot_end.ok_or_else(|| {
            rest_error!("(Occupation) flight plan target_timeslot_end is None.");
            OccupationError::TargetTimeslotEnd
        })?;

        Ok(Occupation {
            flight_plan_id: obj.id,
            vertipad_id: data.target_vertipad_id,
            vertipad_display_name: None,
            time_window: TimeWindow {
                timestamp_min: target_timeslot_start.into(),
                timestamp_max: target_timeslot_end.into(),
            },
            aircraft_id: data.vehicle_id,
            aircraft_nickname: None,
            cargo_acquire: vec![],
            cargo_deliver: vec![],
        })
    }
}

/// Get Regional Vertiports
#[utoipa::path(
    post,
    path = "/cargo/vertiports",
    tag = "svc-cargo",
    request_body = QueryVertiportsRequest,
    responses(
        (status = 200, description = "List all cargo-accessible vertiports successfully", body = [Vertiport]),
        (status = 500, description = "Unable to get vertiports."),
        (status = 503, description = "Could not connect to other microservice dependencies")
    )
)]
#[cfg(not(tarpaulin_include))]
// no_coverage: need backends to test (integration)
pub async fn query_vertiports(
    Extension(grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<QueryVertiportsRequest>,
) -> Result<Json<Vec<Vertiport>>, StatusCode> {
    rest_debug!("(query_vertiports) entry.");

    //
    // 1 degree of latitude ~= 69 miles
    // 1 degree of longitude ~= 55 miles
    //
    let degree_range: f32 = 2.0;
    let filter = AdvancedSearchFilter::search_geo_intersect(
        "geo_location".to_owned(),
        format!(
            "POLYGON((
            {lon_max} {lat_max},
            {lon_min} {lat_max},
            {lon_min} {lat_min},
            {lon_max} {lat_min},
            {lon_max} {lat_max}
        ))",
            lat_max = (payload.latitude + degree_range).to_string(),
            lon_max = (payload.longitude + degree_range).to_string(),
            lat_min = (payload.latitude - degree_range).to_string(),
            lon_min = (payload.longitude - degree_range).to_string(),
        ),
    );

    // Make request, process response
    let vertiports = grpc_clients
        .storage
        .vertiport
        .search(filter)
        .await
        .map_err(|e| {
            rest_error!("(query_vertiports) svc-storage error. {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_inner()
        .list
        .into_iter()
        .filter_map(|vertiport| Vertiport::try_from(vertiport).ok())
        .collect::<Vec<Vertiport>>();

    rest_info!("(query_vertiports) found {} vertiports.", vertiports.len());
    Ok(Json(vertiports))
}

#[derive(Debug, PartialEq)]
pub enum QueryError {
    VertiportId,
    ArrivalWindow,
    Limit,
}

impl Display for QueryError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            QueryError::VertiportId => write!(f, "Invalid vertiport ID"),
            QueryError::ArrivalWindow => write!(f, "Arrival window not specified"),
            QueryError::Limit => {
                write!(f, "Specified limit beyond max of {MAX_LANDINGS_TO_RETURN}")
            }
        }
    }
}

#[derive(Debug)]
struct OccupationsRequest {
    limit: i32,
    vertiport_id: Uuid,
    arrival_window: TimeWindow,
}

fn occupations_request_validation(
    request: QueryScheduleRequest,
) -> Result<OccupationsRequest, QueryError> {
    if request.limit > MAX_LANDINGS_TO_RETURN {
        return Err(QueryError::Limit);
    }

    Ok(OccupationsRequest {
        limit: request.limit as i32,
        vertiport_id: to_uuid(&request.vertiport_id).ok_or(QueryError::VertiportId)?,
        arrival_window: request.arrival_window.ok_or(QueryError::ArrivalWindow)?,
    })
}

/// Request a list of occupations for a vertiport.
/// No more than [`MAX_LANDINGS_TO_RETURN`] occupations will be returned.
#[utoipa::path(
    post,
    path = "/cargo/occupations",
    tag = "svc-cargo",
    responses(
        (status = 200, description = "Occupations retrieved successfully"),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "Dependencies returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    ),
    request_body = QueryScheduleRequest
)]
#[cfg(not(tarpaulin_include))]
// no_coverage: need backends to test (integration)
pub async fn query_occupations(
    Extension(grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<QueryScheduleRequest>,
) -> Result<Json<QueryScheduleResponse>, StatusCode> {
    rest_debug!("(query_occupations) entry.");

    let payload = occupations_request_validation(payload).map_err(|e| {
        rest_error!("(query_occupations) {}", e);
        StatusCode::BAD_REQUEST
    })?;

    //
    // Request flight plans
    //
    let mut filter = AdvancedSearchFilter::search_equals(
        "destination_vertiport_id".to_string(),
        payload.vertiport_id.to_string(),
    )
    .and_between(
        "scheduled_arrival".to_string(),
        payload.arrival_window.timestamp_min.to_string(),
        payload.arrival_window.timestamp_max.to_string(),
    );
    filter.results_per_page = payload.limit;
    filter.order_by = vec![SortOption {
        sort_field: "scheduled_arrival".to_string(),
        sort_order: SortOrder::Asc as i32,
    }];

    let mut occupations = grpc_clients
        .storage
        .flight_plan
        .search(filter)
        .await
        .map_err(|e| {
            rest_error!("(query_occupations) svc-storage error. {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_inner()
        .list
        .into_iter()
        .filter_map(|plan| Occupation::try_from(plan).ok())
        .collect::<Vec<Occupation>>();

    for occupation in &mut occupations {
        occupation.vertipad_display_name =
            match super::utils::get_vertipad_data(&occupation.vertipad_id, &grpc_clients).await {
                Ok(vertipad) => Some(vertipad.name),
                Err(e) => {
                    rest_warn!("(Occupation) couldn't get vertipad display name: {:?}", e);
                    None
                }
            };

        occupation.aircraft_nickname =
            match super::utils::get_vehicle_data(&occupation.aircraft_id, &grpc_clients).await {
                Ok(vehicle) => Some(vehicle.registration_number),
                Err(e) => {
                    rest_warn!("(Occupation) couldn't get vehicle nickname: {:?}", e);
                    None
                }
            };
    }

    Ok(Json(QueryScheduleResponse { occupations }))
}

/// Request a list of scans for a parcel.
#[utoipa::path(
    get,
    path = "/cargo/track/{id}",
    tag = "svc-cargo",
    responses(
        (status = 200, description = "Parcel scans retrieved successfully", body = QueryParcelResponse),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "Dependencies returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    ),
    params(
        ("id" = String, Path, description = "Parcel id"),
    )
)]
#[cfg(not(tarpaulin_include))]
// no_coverage: need backends to test (integration)
pub async fn query_scans(
    Extension(grpc_clients): Extension<GrpcClients>,
    Path(parcel_id): Path<String>,
) -> Result<Json<QueryParcelResponse>, StatusCode> {
    rest_info!("(query_scans) entry.");
    to_uuid(&parcel_id).ok_or_else(|| {
        rest_error!("(query_scans) parcel ID not in UUID format.");
        StatusCode::BAD_REQUEST
    })?;

    //
    // Request parcel scans
    //
    let mut filter =
        AdvancedSearchFilter::search_equals("parcel_id".to_string(), parcel_id.clone());

    filter.order_by = vec![SortOption {
        sort_field: "created_at".to_string(),
        sort_order: SortOrder::Asc as i32,
    }];

    let scans = grpc_clients
        .storage
        .parcel_scan
        .search(filter)
        .await
        .map_err(|e| {
            rest_error!("(query_scans) svc-storage error {:?}", e);
            StatusCode::NOT_FOUND
        })?
        .into_inner()
        .list
        .into_iter()
        .filter_map(|scan| CargoScan::try_from(scan).ok())
        .collect::<Vec<CargoScan>>();

    Ok(Json(QueryParcelResponse { scans }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_common::time::Utc;
    use svc_storage_client_grpc::prelude::GeoPolygon;

    #[test]
    fn test_try_from_vertiport_object() {
        let data = vertiport::mock::get_data_obj();
        let mut object = vertiport::Object {
            id: "123".to_string(),
            data: Some(data.clone()),
        };

        // valid
        Vertiport::try_from(object.clone()).unwrap();

        // invalid data
        object.data = None;
        assert_eq!(
            Vertiport::try_from(object.clone()).unwrap_err(),
            VertiportError::Data
        );

        // invalid location
        let tmp = vertiport::Data {
            geo_location: None,
            ..data.clone()
        };
        object.data = Some(tmp);
        assert_eq!(
            Vertiport::try_from(object.clone()).unwrap_err(),
            VertiportError::Location
        );

        // invalid exterior
        let tmp = vertiport::Data {
            geo_location: Some(GeoPolygon {
                exterior: None,
                interiors: vec![],
            }),
            ..data.clone()
        };
        object.data = Some(tmp);
        assert_eq!(
            Vertiport::try_from(object.clone()).unwrap_err(),
            VertiportError::Exterior
        );
    }

    #[test]
    fn test_try_from_parcel_scan_object() {
        let data = parcel_scan::mock::get_data_obj();
        let mut object = parcel_scan::Object {
            id: "123".to_string(),
            data: Some(data.clone()),
        };

        // valid
        CargoScan::try_from(object.clone()).unwrap();

        // invalid data
        object.data = None;
        assert_eq!(
            CargoScan::try_from(object.clone()).unwrap_err(),
            ScanError::Data
        );

        // invalid location
        let tmp = parcel_scan::Data {
            geo_location: None,
            ..data.clone()
        };
        object.data = Some(tmp);
        assert_eq!(
            CargoScan::try_from(object.clone()).unwrap_err(),
            ScanError::Location
        );

        // invalid created_at
        let tmp = parcel_scan::Data {
            created_at: None,
            ..data.clone()
        };
        object.data = Some(tmp);
        assert_eq!(
            CargoScan::try_from(object.clone()).unwrap_err(),
            ScanError::CreatedAt
        );
    }

    #[test]
    fn test_try_from_flight_plan_object() {
        let data = flight_plan::mock::get_data_obj();
        let mut object = flight_plan::Object {
            id: "123".to_string(),
            data: Some(data.clone()),
        };

        // valid
        Occupation::try_from(object.clone()).unwrap();

        // invalid data
        object.data = None;
        assert_eq!(
            Occupation::try_from(object.clone()).unwrap_err(),
            OccupationError::Data
        );

        // invalid target_timeslot_start
        let tmp = flight_plan::Data {
            target_timeslot_start: None,
            ..data.clone()
        };
        object.data = Some(tmp);
        assert_eq!(
            Occupation::try_from(object.clone()).unwrap_err(),
            OccupationError::TargetTimeslotStart
        );

        // invalid target_timeslot_end
        let tmp = flight_plan::Data {
            target_timeslot_end: None,
            ..data.clone()
        };
        object.data = Some(tmp);
        assert_eq!(
            Occupation::try_from(object.clone()).unwrap_err(),
            OccupationError::TargetTimeslotEnd
        );
    }

    #[test]
    fn test_query_occupations() {
        // invalid vertiport ID
        let request = QueryScheduleRequest {
            vertiport_id: "invalid".to_string(),
            arrival_window: Some(TimeWindow {
                timestamp_min: Utc::now(),
                timestamp_max: Utc::now(),
            }),
            limit: MAX_LANDINGS_TO_RETURN,
        };

        let response = occupations_request_validation(request).unwrap_err();
        assert_eq!(response, QueryError::VertiportId);

        // invalid limit
        let request = QueryScheduleRequest {
            vertiport_id: "123".to_string(),
            arrival_window: Some(TimeWindow {
                timestamp_min: Utc::now(),
                timestamp_max: Utc::now(),
            }),
            limit: MAX_LANDINGS_TO_RETURN + 1,
        };

        let response = occupations_request_validation(request).unwrap_err();
        assert_eq!(response, QueryError::Limit);

        // invalid arrival window
        let request = QueryScheduleRequest {
            vertiport_id: Uuid::new_v4().to_string(),
            arrival_window: None,
            limit: MAX_LANDINGS_TO_RETURN,
        };

        let response = occupations_request_validation(request).unwrap_err();
        assert_eq!(response, QueryError::ArrivalWindow);

        // valid
        let expected_time_window = TimeWindow {
            timestamp_min: Utc::now(),
            timestamp_max: Utc::now(),
        };

        let request = QueryScheduleRequest {
            vertiport_id: Uuid::new_v4().to_string(),
            arrival_window: Some(expected_time_window),
            limit: MAX_LANDINGS_TO_RETURN,
        };

        let result = occupations_request_validation(request.clone()).unwrap();
        assert_eq!(
            result.arrival_window.timestamp_max,
            expected_time_window.timestamp_max
        );
        assert_eq!(
            result.arrival_window.timestamp_min,
            expected_time_window.timestamp_min
        );
        assert_eq!(
            result.vertiport_id,
            Uuid::parse_str(&request.vertiport_id).unwrap()
        );
        assert_eq!(result.limit as u32, request.limit);
    }

    #[tokio::test]
    async fn test_query_scans() {
        let config = crate::config::Config::default();
        let grpc_clients = GrpcClients::default(config);

        // invalid parcel ID
        let result = query_scans(Extension(grpc_clients.clone()), Path("invalid".to_string()))
            .await
            .unwrap_err();
        assert_eq!(result, StatusCode::BAD_REQUEST);

        // valid
        let parcel_id = Uuid::new_v4().to_string();
        let _ = query_scans(Extension(grpc_clients.clone()), Path(parcel_id.clone()))
            .await
            .unwrap();
    }

    #[test]
    fn test_query_error_display() {
        assert_eq!(
            format!("{}", QueryError::VertiportId),
            "Invalid vertiport ID"
        );
        assert_eq!(
            format!("{}", QueryError::ArrivalWindow),
            "Arrival window not specified"
        );
        assert_eq!(
            format!("{}", QueryError::Limit),
            format!("Specified limit beyond max of {MAX_LANDINGS_TO_RETURN}")
        );
    }
}
