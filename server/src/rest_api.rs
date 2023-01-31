pub mod rest_types {
    include!("../../openapi/types.rs");
}

use axum::{extract::Extension, Json};
use chrono::Utc;
use hyper::{HeaderMap, StatusCode};
use lib_common::time::{datetime_to_timestamp, timestamp_to_datetime};
use uuid::Uuid;

use crate::grpc_clients::GrpcClients;

use svc_pricing_client::pricing_grpc::{
    pricing_request::ServiceType, PricingRequest, PricingRequests,
};

use svc_scheduler_client_grpc::grpc::{
    Id, QueryFlightPlan, QueryFlightPlanBundle, QueryFlightRequest,
};

use svc_storage_client_grpc::client::SearchFilter;

pub use rest_types::{
    FlightCancel, FlightConfirm, FlightLeg, FlightQuery, Itinerary, Vertiport, VertiportsQuery,
};

/// Writes an info! message to the app::req logger
macro_rules! req_info {
    ($($arg:tt)+) => {
        log::info!(target: "app::req", $($arg)+);
    };
}

/// Writes an error! message to the app::req logger
macro_rules! req_error {
    ($($arg:tt)+) => {
        log::error!(target: "app::req", $($arg)+);
    };
}

/// Writes a debug! message to the app::req logger
macro_rules! req_debug {
    ($($arg:tt)+) => {
        log::debug!(target: "app::req", $($arg)+);
    };
}

/// Don't allow excessively heavy loads
const MAX_CARGO_WEIGHT_G: u32 = 1_000_000; // 1000 kg

/// Don't allow large UUID strings
const UUID_MAX_SIZE: usize = 50; // Sometimes braces or hyphens

/// Returns true if a given string is UUID format
fn is_uuid(s: &str) -> bool {
    // Prevent buffer overflows
    if s.len() > UUID_MAX_SIZE {
        req_error!("(is_uuid) input string larger than expected: {}.", s.len());
        return false;
    }

    Uuid::parse_str(s).is_ok()
}

/// Parses the incoming flight plans for information the customer wants
fn parse_flight(plan: &QueryFlightPlan) -> Option<FlightLeg> {
    let Some(prost_time) = plan.estimated_departure.clone() else {
        let error_msg = "no departure time in flight plan; discarding.";
        req_error!("(parse_flight) {}", &error_msg);
        return None;
    };

    let Some(time_depart) = timestamp_to_datetime(&prost_time) else {
        let error_msg = "can't convert prost timestamp to datetime.";
        req_error!("(parse_flight) {}", &error_msg);
        return None;
    };

    let Some(prost_time) = plan.estimated_arrival.clone() else {
        let error_msg = "(parse_flight) no arrival time in flight plan; discarding.";
        req_error!("(parse_flight) {}", &error_msg);
        return None;
    };

    let Some(time_arrive) = timestamp_to_datetime(&prost_time) else {
        let error_msg = "can't convert prost timestamp to datetime.";
        req_error!("(parse_flight) {}", &error_msg);
        return None;
    };

    Some(FlightLeg {
        fp_id: plan.id.clone(),
        vertiport_depart_id: plan.vertiport_depart_id.to_string(),
        vertiport_arrive_id: plan.vertiport_arrive_id.to_string(),
        timestamp_depart: time_depart,
        timestamp_arrive: time_arrive,
        distance_m: plan.estimated_distance as f32,
        base_pricing: None,
        currency_type: None,
    })
}

/// Get Regional Vertiports
#[utoipa::path(
    post,
    path = "/cargo/vertiports",
    tag = "svc-cargo",
    request_body = VertiportsQuery,
    responses(
        (status = 200, description = "List all cargo-accessible vertiports successfully", body = [Vertiport]),
        (status = 500, description = "Unable to get vertiports."),
        (status = 503, description = "Could not connect to other microservice dependencies")
    )
)]
pub async fn query_vertiports(
    Extension(mut grpc_clients): Extension<GrpcClients>,
    Json(_payload): Json<VertiportsQuery>,
) -> Result<Json<Vec<Vertiport>>, (StatusCode, String)> {
    req_debug!("(query_vertiports) entry.");

    // Will provide Lat, Long
    let request = tonic::Request::new(SearchFilter {
        search_field: "".to_string(),
        search_value: "".to_string(),
        page_number: 1,
        results_per_page: 50,
    });

    // Get Client
    let client_option = grpc_clients.storage.get_client().await;
    if client_option.is_none() {
        let error_msg = "svc-storage unavailable.".to_string();
        req_error!("(query_vertiports) {}", &error_msg);
        return Err((StatusCode::SERVICE_UNAVAILABLE, error_msg));
    }
    let mut client = client_option.unwrap();

    // Make request, process response
    let response = client.get_all_with_filter(request).await;
    match response {
        Ok(r) => {
            let ret: Vec<Vertiport> = r
                .into_inner()
                .list
                .into_iter()
                .map(|x| {
                    let data = x.data.unwrap();
                    Vertiport {
                        id: x.id,
                        label: data.description,
                        latitude: data.latitude,
                        longitude: data.longitude,
                    }
                })
                .collect();

            req_info!("(query_vertiports) found {} vertiports.", ret.len());
            Ok(Json(ret))
        }
        Err(e) => {
            let error_msg = "error response from svc-storage.".to_string();
            req_error!("(query_vertiports) {} {:?}", &error_msg, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_msg))
        }
    }
}

/// Get Available Flights
///
/// Search for available trips and return a list of [`Itinerary`].
#[utoipa::path(
    post,
    path = "/cargo/query",
    tag = "svc-cargo",
    request_body = FlightQuery,
    responses(
        (status = 200, description = "List available flight plans", body = [Itinerary]),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "svc-scheduler or svc-pricing returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    )
)]
pub async fn query_flight(
    Extension(mut grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<FlightQuery>,
) -> Result<Json<Vec<Itinerary>>, (StatusCode, String)> {
    req_debug!("(query_flight) entry.");

    //
    // Validate Request
    //

    // Reject extreme weights
    let weight_g: u32 = (payload.cargo_weight_kg * 1000.0) as u32;
    if weight_g >= MAX_CARGO_WEIGHT_G {
        let error_msg = format!("request cargo weight exceeds {MAX_CARGO_WEIGHT_G}.");
        req_error!("(query_flight) {}", &error_msg);
        return Err((StatusCode::BAD_REQUEST, error_msg));
    }

    // Check UUID validity
    if !is_uuid(&payload.vertiport_arrive_id) {
        let error_msg = "arrival port ID not UUID format.".to_string();
        req_error!("(query_flight) {}", &error_msg);
        return Err((StatusCode::BAD_REQUEST, error_msg));
    }

    if !is_uuid(&payload.vertiport_depart_id) {
        let error_msg = "departure port ID not UUID format.".to_string();
        req_error!("(query_flight) {}", &error_msg);
        return Err((StatusCode::BAD_REQUEST, error_msg));
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
            req_error!("(query_flight) {} {:?}", &error_msg, window.timestamp_max);
            return Err((StatusCode::BAD_REQUEST, error_msg));
        }

        let Some(ts) = datetime_to_timestamp(&window.timestamp_max) else {
            let error_msg = "unable to convert datetime to timestamp.".to_string();
            req_error!("(query_flight) {} {:?}", &error_msg, window.timestamp_max);
            return Err((StatusCode::BAD_REQUEST, error_msg));
        };

        flight_query.latest_arrival_time = Some(ts);
    }

    if let Some(window) = payload.time_depart_window {
        if window.timestamp_max <= current_time {
            let error_msg = "max depart time is in the past.".to_string();
            req_error!("(query_flight) {} {:?}", &error_msg, window.timestamp_max);
            return Err((StatusCode::BAD_REQUEST, error_msg));
        }

        let Some(ts) = datetime_to_timestamp(&window.timestamp_max) else {
            let error_msg = "unable to convert datetime to timestamp.".to_string();
            req_error!("(query_flight) {} {:?}", &error_msg, window.timestamp_max);
            return Err((StatusCode::BAD_REQUEST, error_msg));
        };

        flight_query.earliest_departure_time = Some(ts);
    }

    if flight_query.earliest_departure_time.is_none() && flight_query.latest_arrival_time.is_none()
    {
        let error_msg = "invalid time window.".to_string();
        req_error!("(query_flight) {}", &error_msg);
        return Err((StatusCode::BAD_REQUEST, error_msg));
    }

    //
    // GRPC Request
    //

    let Some(mut scheduler_client) = grpc_clients.scheduler.get_client().await else {
        let error_msg = "svc-scheduler unavailable.".to_string();
        req_error!("(query_flight) {}", &error_msg);
        return Err((StatusCode::SERVICE_UNAVAILABLE, error_msg));
    };

    let Some(mut pricing_client) = grpc_clients.pricing.get_client().await else {
        let error_msg = "svc-pricing unavailable.".to_string();
        req_error!("(query_flight) {}", &error_msg);
        return Err((StatusCode::SERVICE_UNAVAILABLE, error_msg));
    };

    let request = tonic::Request::new(flight_query);
    let response = scheduler_client.query_flight(request).await;
    let Ok(response) = response else {
        let error_msg = "svc-scheduler error.".to_string();
        req_error!("(query_flight) {} {:?}", &error_msg, response.unwrap_err());
        req_error!("(query_flight) invalidating svc-scheduler client.");
        grpc_clients.scheduler.invalidate().await;
        return Err((StatusCode::INTERNAL_SERVER_ERROR, error_msg));
    };
    let bundles: Vec<QueryFlightPlanBundle> = response.into_inner().flights;

    //
    // Unpack flight itineraries
    //

    // List of lists of flights
    let mut itineraries: Vec<Itinerary> = vec![];
    for b in &bundles {
        let mut legs: Vec<FlightLeg> = vec![];

        if let Some(plan) = &b.flight_plan {
            if let Some(leg) = parse_flight(plan) {
                legs.push(leg);
            }
        }

        for dh in &b.deadhead_flight_plans {
            if let Some(leg) = parse_flight(dh) {
                legs.push(leg);
            }
        }

        itineraries.push(Itinerary {
            legs,
            base_pricing: None,
            currency_type: Some("usd".to_string()),
        })
    }
    req_info!("(query_flight) found {} flight options.", itineraries.len());

    //
    // Get pricing for each itinerary
    //

    // StatusUpdate message to customer?
    // e.g. Got your flights! Calculating prices...
    for mut itinerary in &mut itineraries {
        let mut pricing_requests = PricingRequests { requests: vec![] };

        for leg in &itinerary.legs {
            let pricing_query = PricingRequest {
                service_type: ServiceType::Cargo as i32,
                distance_km: leg.distance_m / 1000.0,
                weight_kg: payload.cargo_weight_kg,
            };

            pricing_requests.requests.push(pricing_query);
        }

        // Make request, process response
        let request = tonic::Request::new(pricing_requests);
        let response = pricing_client.get_pricing(request).await;

        let Ok(response) = response else {
            let error_msg = "svc-pricing error.".to_string();
            req_error!("(query_flight) {} {:?}", &error_msg, response.unwrap_err());
            req_error!("(query_flight) invalidating svc-pricing client.");
            grpc_clients.pricing.invalidate().await;
            return Err((StatusCode::INTERNAL_SERVER_ERROR, error_msg));
        };

        let response = response.into_inner();

        for (price, mut leg) in response.prices.iter().zip(itinerary.legs.iter_mut()) {
            leg.base_pricing = Some(*price);
            leg.currency_type = Some("usd".to_string());
        }

        itinerary.base_pricing = Some(response.prices.iter().sum());
    }

    req_debug!(
        "(query_flight) exit with {} itineraries.",
        itineraries.len()
    );
    Ok(Json(itineraries))
}

/// Confirm a Flight
#[utoipa::path(
    put,
    path = "/cargo/confirm",
    tag = "svc-cargo",
    request_body = FlightConfirm,
    responses(
        (status = 200, description = "Flight Confirmed", body = String),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "svc-scheduler returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    )
)]
pub async fn confirm_flight(
    Extension(mut grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<FlightConfirm>,
    _headers: HeaderMap,
) -> Result<String, (StatusCode, String)> {
    req_debug!("(confirm_flight) entry.");

    if !is_uuid(&payload.fp_id) {
        let error_msg = "flight plan ID not in UUID format.".to_string();
        req_error!("(confirm_flight) {}", &error_msg);
        return Err((StatusCode::BAD_REQUEST, error_msg));
    }

    // Get Client
    let client_option = grpc_clients.scheduler.get_client().await;
    if client_option.is_none() {
        let error_msg = "svc-scheduler unavailable.".to_string();
        req_error!("(confirm_flight) {}", &error_msg);
        return Err((StatusCode::SERVICE_UNAVAILABLE, error_msg));
    }
    let mut client = client_option.unwrap();

    // Make request, process response
    let request = tonic::Request::new(Id { id: payload.fp_id });
    let response = client.confirm_flight(request).await;
    match response {
        Ok(r) => {
            let ret = r.into_inner();
            if ret.confirmed {
                req_info!("(confirm_flight) svc-scheduler confirm success.");
                Ok(ret.id)
            } else {
                let error_msg = "svc-scheduler confirm fail.".to_string();
                req_error!("(confirm_flight) {}", &error_msg);
                Err((StatusCode::INTERNAL_SERVER_ERROR, error_msg))
            }
        }
        Err(e) => {
            let error_msg = "svc-scheduler error.".to_string();
            req_error!("(confirm_flight) {} {:?}", &error_msg, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_msg))
        }
    }
}

/// Cancel a Flight
#[utoipa::path(
    delete,
    path = "/cargo/cancel",
    tag = "svc-cargo",
    responses(
        (status = 200, description = "Flight cancelled successfully"),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "svc-scheduler returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    ),
    request_body = FlightCancel
)]
pub async fn cancel_flight(
    Extension(mut grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<FlightCancel>,
    _headers: HeaderMap,
) -> Result<String, (StatusCode, String)> {
    req_debug!("(cancel_flight) entry.");
    if !is_uuid(&payload.fp_id) {
        let error_msg = "flight plan ID not in UUID format.".to_string();
        req_error!("(cancel_flight) {}", &error_msg);
        return Err((StatusCode::BAD_REQUEST, error_msg));
    }

    // Get Client
    let client_option = grpc_clients.scheduler.get_client().await;
    if client_option.is_none() {
        let error_msg = "svc-scheduler unavailable.".to_string();
        req_error!("(cancel_flight) {}", &error_msg);
        return Err((StatusCode::SERVICE_UNAVAILABLE, error_msg));
    }
    let mut client = client_option.unwrap();

    // Make request, process response
    let request = tonic::Request::new(Id { id: payload.fp_id });
    let response = client.cancel_flight(request).await;
    match response {
        Ok(r) => {
            let ret = r.into_inner();
            if ret.cancelled {
                req_info!("(cancel_flight) svc-scheduler cancel success.");
                Ok(ret.id)
            } else {
                let error_msg = "svc-scheduler cancel fail.".to_string();
                req_error!("(cancel_flight) {} {}", &error_msg, ret.reason);
                Err((StatusCode::INTERNAL_SERVER_ERROR, error_msg))
            }
        }
        Err(e) => {
            let error_msg = "svc-scheduler request fail.".to_string();
            req_error!("(cancel_flight) {} {:?}", &error_msg, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_msg))
        }
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn ut_parse_flight() {
        let depart_time = datetime_to_timestamp(&Utc::now());
        assert!(depart_time.is_some());

        let fp = QueryFlightPlan {
            id: "".to_string(),
            pilot_id: "".to_string(),
            vehicle_id: "".to_string(),
            cargo: [123].to_vec(),
            weather_conditions: "Sunny, no wind :)".to_string(),
            vertiport_depart_id: "".to_string(),
            pad_depart_id: "".to_string(),
            vertiport_arrive_id: "".to_string(),
            pad_arrive_id: "".to_string(),
            estimated_departure: depart_time.clone(),
            estimated_arrival: depart_time,
            actual_departure: None,
            actual_arrival: None,
            flight_release_approval: None,
            flight_plan_submitted: None,
            flight_status: 0,
            flight_priority: 0,
            estimated_distance: 1000,
        };
        let ret = parse_flight(&fp);
        assert!(ret.is_some());
        let opt = ret.unwrap();
        assert_eq!(fp.id, opt.fp_id);
        assert_eq!(fp.vertiport_depart_id, opt.vertiport_depart_id);
        assert_eq!(fp.vertiport_arrive_id, opt.vertiport_arrive_id);

        // Bad time arguments
        {
            let mut fp2 = fp.clone();
            fp2.estimated_departure = None;
            assert!(parse_flight(&fp2).is_none());
        }

        {
            let mut fp2 = fp.clone();
            fp2.estimated_arrival = None;
            assert!(parse_flight(&fp2).is_none());
        }
    }
}
