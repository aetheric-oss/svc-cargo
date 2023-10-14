use super::utils::is_uuid;
use crate::grpc::client::GrpcClients;
use crate::rest_types::{FlightLeg, FlightRequest, Itinerary};
use axum::{extract::Extension, Json};
use chrono::{Duration, Utc};
use geo::HaversineDistance;
use hyper::StatusCode;
use lib_common::grpc::Client;

//
// Other Service Dependencies
//
use svc_pricing_client_grpc::client::{
    pricing_request::ServiceType, PricingRequest, PricingRequests,
};
use svc_pricing_client_grpc::service::Client as ServiceClient;
use svc_scheduler_client_grpc::client::{Itinerary as SchedulerItinerary, QueryFlightRequest};
use svc_scheduler_client_grpc::prelude::scheduler_storage::{
    flight_plan::Object as FlightPlanObject, GeoPoint,
};
use svc_scheduler_client_grpc::prelude::SchedulerServiceClient;

/// Don't allow excessively heavy loads
const MAX_CARGO_WEIGHT_G: u32 = 1_000_000; // 1000 kg

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FlightPlanError {
    DepartureTime,
    ArrivalTime,
    Data,
    Path,
}

/// Gets the total distance of a path in meters
/// TODO(R4): Temporary function to convert path to distance, until svc-storage is updated with it
fn get_distance_meters(path: &[GeoPoint]) -> Result<f32, FlightPlanError> {
    let mut distance: f64 = 0.0;
    if path.len() < 2 {
        rest_error!(
            "(get_distance_meters) path too short: {} leg(s).",
            path.len()
        );
        return Err(FlightPlanError::Path);
    }

    let it = path.windows(2);
    for pair in it {
        let (p1, p2) = (
            geo::point!(
                x: pair[0].longitude,
                y: pair[0].latitude
            ),
            geo::point!(
                x: pair[1].longitude,
                y: pair[1].latitude
            ),
        );

        distance += p1.haversine_distance(&p2);
    }

    Ok(distance as f32)
}

impl TryFrom<FlightPlanObject> for FlightLeg {
    type Error = FlightPlanError;

    fn try_from(plan: FlightPlanObject) -> Result<Self, Self::Error> {
        let msg_prefix = "(FlightLeg::try_from(FlightPlanObject))";

        let Some(data) = plan.data.clone() else {
            let error_msg = "no data in flight plan; discarding.";
            rest_error!("{msg_prefix} {}", &error_msg);
            return Err(FlightPlanError::Data);
        };

        let Some(timestamp_depart) = data.scheduled_departure.clone() else {
            let error_msg = "no departure time in flight plan; discarding.";
            rest_error!("{msg_prefix} {}", &error_msg);
            return Err(FlightPlanError::DepartureTime);
        };

        let Some(timestamp_arrive) = data.scheduled_arrival.clone() else {
            let error_msg = "{msg_prefix} no arrival time in flight plan; discarding.";
            rest_error!("{msg_prefix} {}", &error_msg);
            return Err(FlightPlanError::ArrivalTime);
        };

        let Some(vertiport_depart_id) = data.departure_vertiport_id.clone() else {
            let error_msg = "{msg_prefix} no departure vertiport in flight plan; discarding.";
            rest_error!("{msg_prefix} {}", &error_msg);
            return Err(FlightPlanError::DepartureTime);
        };

        let Some(vertiport_arrive_id) = data.destination_vertiport_id.clone() else {
            let error_msg = "{msg_prefix} no arrival vertiport in flight plan; discarding.";
            rest_error!("{msg_prefix} {}", &error_msg);
            return Err(FlightPlanError::ArrivalTime);
        };

        let path = match data.path {
            Some(path) => path.points,
            _ => {
                let error_msg = "{msg_prefix} no path in flight plan; discarding.";
                rest_error!("{msg_prefix} {}", &error_msg);
                return Err(FlightPlanError::Data);
            }
        };

        let distance_meters = get_distance_meters(&path)?;

        Ok(FlightLeg {
            flight_plan_id: plan.id,
            vertiport_depart_id,
            vertiport_arrive_id,
            timestamp_depart: timestamp_depart.into(),
            timestamp_arrive: timestamp_arrive.into(),
            path,
            distance_meters,
            base_pricing: None,
            currency_type: None,
        })
    }
}

// Get Available Flights
///
/// Search for available trips and return a list of [`Itinerary`].
#[utoipa::path(
    post,
    path = "/cargo/request",
    tag = "svc-cargo",
    request_body = FlightRequest,
    responses(
        (status = 200, description = "List available flight plans", body = [Itinerary]),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "svc-scheduler or svc-pricing returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    )
)]
pub async fn request_flight(
    Extension(mut grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<FlightRequest>,
) -> Result<Json<Vec<Itinerary>>, StatusCode> {
    rest_debug!("(request_flight) entry.");

    //
    // Validate Request
    //

    // Reject extreme weights
    let weight_g: u32 = (payload.cargo_weight_kg * 1000.0) as u32;
    if weight_g >= MAX_CARGO_WEIGHT_G {
        let error_msg = format!("request cargo weight exceeds {MAX_CARGO_WEIGHT_G}.");
        rest_error!("(request_flight) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check UUID validity
    if !is_uuid(&payload.vertiport_arrive_id) {
        let error_msg = "arrival port ID not UUID format.".to_string();
        rest_error!("(request_flight) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    if !is_uuid(&payload.vertiport_depart_id) {
        let error_msg = "departure port ID not UUID format.".to_string();
        rest_error!("(request_flight) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut flight_query = QueryFlightRequest {
        is_cargo: true,
        persons: None,
        weight_grams: Some(weight_g),
        vertiport_depart_id: payload.vertiport_depart_id,
        vertiport_arrive_id: payload.vertiport_arrive_id,
        earliest_departure_time: None,
        latest_arrival_time: None,
    };

    let current_time = Utc::now();

    // Time windows are properly specified
    if let Some(window) = payload.time_arrive_window {
        if window.timestamp_max <= current_time {
            let error_msg = "max arrival time is in the past.".to_string();
            rest_error!("(request_flight) {} {:?}", &error_msg, window.timestamp_max);
            return Err(StatusCode::BAD_REQUEST);
        }

        // TODO(R4) - Rework this interface to be more intuitive
        flight_query.earliest_departure_time =
            Some((window.timestamp_min - Duration::hours(2)).into());
        flight_query.latest_arrival_time = Some(window.timestamp_max.into());
    }

    if let Some(window) = payload.time_depart_window {
        if window.timestamp_max <= current_time {
            let error_msg = "max depart time is in the past.".to_string();
            rest_error!("(request_flight) {} {:?}", &error_msg, window.timestamp_max);
            return Err(StatusCode::BAD_REQUEST);
        }

        // TODO(R4) - Rework this interface to be more intuitive
        flight_query.earliest_departure_time = Some(window.timestamp_min.into());
        flight_query.latest_arrival_time = Some((window.timestamp_max + Duration::hours(2)).into());
    }

    if flight_query.earliest_departure_time.is_none() || flight_query.latest_arrival_time.is_none()
    {
        let error_msg = "invalid time window.".to_string();
        rest_error!("(request_flight) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    //
    // GRPC Request
    //
    let response = grpc_clients.scheduler.query_flight(flight_query).await;
    let Ok(response) = response else {
        let error_msg = "svc-scheduler error.".to_string();
        rest_error!("(request_flight) {} {:?}", &error_msg, response.unwrap_err());
        rest_error!("(request_flight) invalidating svc-scheduler client.");
        grpc_clients.scheduler.invalidate().await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let itineraries: Vec<SchedulerItinerary> = response.into_inner().itineraries;

    //
    // Unpack flight itineraries
    //

    // List of lists of flights
    let mut offerings: Vec<Itinerary> = vec![];
    for itinerary in itineraries.into_iter() {
        let id = itinerary.id.clone();
        let legs = itinerary
            .flight_plans
            .into_iter()
            .map(FlightLeg::try_from)
            .collect::<Result<Vec<FlightLeg>, FlightPlanError>>();

        let Ok(legs) = legs else {
            rest_error!("(request_flight) Itinerary contained invalid flight plan(s).");
            continue;
        };

        offerings.push(Itinerary {
            id,
            legs,
            base_pricing: None,
            // TODO(R4): Vary currency by region
            currency_type: Some("usd".to_string()),
        })
    }
    rest_info!("(request_flight) found {} flight options.", offerings.len());

    //
    // Get pricing for each itinerary
    //

    // StatusUpdate message to customer?
    // e.g. Got your flights! Calculating prices...
    for mut itinerary in &mut offerings {
        let mut pricing_requests = PricingRequests { requests: vec![] };

        for leg in &itinerary.legs {
            let pricing_query = PricingRequest {
                service_type: ServiceType::Cargo as i32,
                distance_km: leg.distance_meters / 1000.0,
                weight_kg: payload.cargo_weight_kg,
            };

            pricing_requests.requests.push(pricing_query);
        }

        // Make request, process response
        let response = grpc_clients.pricing.get_pricing(pricing_requests).await;

        let Ok(response) = response else {
            let error_msg = "svc-pricing error.".to_string();
            rest_error!("(request_flight) {} {:?}", &error_msg, response.unwrap_err());
            rest_error!("(request_flight) invalidating svc-pricing client.");
            grpc_clients.pricing.invalidate().await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        let response = response.into_inner();

        for (price, mut leg) in response.prices.iter().zip(itinerary.legs.iter_mut()) {
            leg.base_pricing = Some(*price);
            leg.currency_type = Some("usd".to_string());
        }

        itinerary.base_pricing = Some(response.prices.iter().sum());
    }

    rest_debug!(
        "(request_flight) exit with {} itineraries.",
        offerings.len()
    );
    Ok(Json(offerings))
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use chrono::{Duration, Utc};
    use svc_scheduler_client_grpc::prelude::scheduler_storage::flight_plan;
    use svc_scheduler_client_grpc::prelude::scheduler_storage::GeoLineString;
    use uuid::Uuid;

    #[test]
    fn ut_flight_plan_object_to_return_type() {
        let data = flight_plan::Data {
            pilot_id: Uuid::new_v4().to_string(),
            vehicle_id: Uuid::new_v4().to_string(),
            departure_vertiport_id: Some(Uuid::new_v4().to_string()),
            destination_vertiport_id: Some(Uuid::new_v4().to_string()),
            departure_vertipad_id: Uuid::new_v4().to_string(),
            destination_vertipad_id: Uuid::new_v4().to_string(),
            path: Some(GeoLineString {
                points: vec![
                    GeoPoint {
                        latitude: 52.37488619450752,
                        longitude: 4.916048576268328,
                    },
                    GeoPoint {
                        latitude: 52.37488619450752,
                        longitude: 4.916048576268328,
                    },
                ],
            }),
            scheduled_departure: Some(Utc::now().into()),
            scheduled_arrival: Some((Utc::now() + Duration::hours(1)).into()),
            ..Default::default()
        };

        let flight_plan = flight_plan::Object {
            id: Uuid::new_v4().to_string(),
            data: Some(data.clone()),
        };

        let leg: FlightLeg = FlightLeg::try_from(flight_plan.clone()).unwrap();
        assert_eq!(flight_plan.id, leg.flight_plan_id);

        let result_data = data.clone();
        assert_eq!(
            result_data.departure_vertiport_id.unwrap(),
            leg.vertiport_depart_id
        );
        assert_eq!(
            result_data.destination_vertiport_id.unwrap(),
            leg.vertiport_arrive_id
        );
        assert_eq!(result_data.path.unwrap().points, leg.path);

        // Bad time arguments
        {
            let mut data = data.clone();
            data.scheduled_departure = None;
            let fp = flight_plan::Object {
                id: Uuid::new_v4().to_string(),
                data: Some(data),
            };
            let e = FlightLeg::try_from(fp).unwrap_err();
            assert_eq!(e, FlightPlanError::DepartureTime);
        }

        {
            let mut data = data.clone();
            data.scheduled_arrival = None;
            let fp = flight_plan::Object {
                id: Uuid::new_v4().to_string(),
                data: Some(data),
            };
            let e = FlightLeg::try_from(fp).unwrap_err();
            assert_eq!(e, FlightPlanError::ArrivalTime);
        }

        {
            let mut data = data.clone();
            // Needs 2 or more points to be valid
            data.path = Some(GeoLineString {
                points: vec![GeoPoint {
                    latitude: 52.37488619450752,
                    longitude: 4.916048576268328,
                }],
            });
            let fp = flight_plan::Object {
                id: Uuid::new_v4().to_string(),
                data: Some(data),
            };
            let e = FlightLeg::try_from(fp).unwrap_err();
            assert_eq!(e, FlightPlanError::Path);
        }

        {
            let fp = flight_plan::Object {
                id: Uuid::new_v4().to_string(),
                data: None,
            };

            let e = FlightLeg::try_from(fp).unwrap_err();
            assert_eq!(e, FlightPlanError::Data);
        }
    }
}
