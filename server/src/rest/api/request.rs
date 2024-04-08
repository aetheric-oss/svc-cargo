pub use super::rest_types::{
    DraftItinerary, FlightPlan, InvoiceItem, Itinerary, QueryItineraryRequest,
};
use super::utils::is_uuid;
use crate::cache::pool::ItineraryPool;
use crate::grpc::client::GrpcClients;
use axum::{extract::Extension, Json};
use chrono::{Duration, Utc};
use geo::HaversineDistance;
use hyper::StatusCode;

//
// Other Service Dependencies
//
use crate::rest::rest_types::CurrencyUnit;
use std::collections::HashMap;
use svc_pricing_client_grpc::prelude::*;
use svc_scheduler_client_grpc::client::Itinerary as SchedulerItinerary;
use svc_scheduler_client_grpc::client::QueryFlightRequest;
use svc_scheduler_client_grpc::prelude::scheduler_storage::GeoPoint;
use svc_scheduler_client_grpc::prelude::FlightPriority;
use svc_scheduler_client_grpc::prelude::SchedulerServiceClient;
use svc_storage_client_grpc::prelude::Id;
use svc_storage_client_grpc::simple_service::Client;

/// Don't allow excessively heavy loads
const MAX_CARGO_WEIGHT_G: u32 = 1_000_000; // 1000 kg

/// Advance notice required
const ADVANCE_NOTICE_MINUTES: i64 = 5;

/// Errors that can occur when processing a flight plan from the scheduler
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FlightPlanError {
    /// Invalid path data
    Path,
}

/// Gets the total distance of a path in meters
/// TODO(R4): Temporary function to convert path to distance, until svc-storage is updated with it
fn get_distance_meters(path: &[GeoPoint]) -> Result<f32, FlightPlanError> {
    // let mut distance: f64 = 0.0;
    if path.len() < 2 {
        rest_error!(
            "(get_distance_meters) path too short: {} segment(s).",
            path.len()
        );
        return Err(FlightPlanError::Path);
    }

    let distance: f64 = path
        .windows(2)
        .map(|pair| {
            geo::point!(
                x: pair[0].longitude,
                y: pair[0].latitude
            )
            .haversine_distance(&geo::point!(
                x: pair[1].longitude,
                y: pair[1].latitude
            ))
        })
        .sum();

    Ok(distance as f32)
}

/// Confirms that a payload has valid fields
fn validate_payload(payload: &QueryItineraryRequest) -> Result<(), StatusCode> {
    // Reject extreme weights
    if payload.cargo_weight_g >= MAX_CARGO_WEIGHT_G {
        let error_msg = format!("request cargo weight exceeds {MAX_CARGO_WEIGHT_G}.");
        rest_error!("(request_flight) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check UUID validity
    if !is_uuid(&payload.target_vertiport_id) {
        let error_msg = "arrival port ID not UUID format.".to_string();
        rest_error!("(request_flight) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    if !is_uuid(&payload.origin_vertiport_id) {
        let error_msg = "departure port ID not UUID format.".to_string();
        rest_error!("(request_flight) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    let Some(ref time_window) = payload.time_depart_window else {
        let error_msg = "missing departure time window.".to_string();
        rest_error!("(request_flight) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    };

    if time_window.timestamp_min >= time_window.timestamp_max {
        let error_msg = "invalid departure time window.".to_string();
        rest_error!("(request_flight) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(())
}

/// Query the scheduler for flight plans
async fn scheduler_query(
    payload: &QueryItineraryRequest,
    grpc_clients: &mut GrpcClients,
) -> Result<Vec<SchedulerItinerary>, StatusCode> {
    let mut flight_query = QueryFlightRequest {
        is_cargo: true,
        persons: None,
        weight_grams: Some(payload.cargo_weight_g),
        origin_vertiport_id: payload.origin_vertiport_id.clone(),
        target_vertiport_id: payload.target_vertiport_id.clone(),
        earliest_departure_time: None,
        latest_arrival_time: None,
        priority: FlightPriority::Low as i32,
    };

    let current_time = Utc::now();

    // Time windows are properly specified
    if let Some(window) = payload.time_arrive_window {
        if window.timestamp_max <= current_time {
            let error_msg = "max arrival time is in the past.".to_string();
            rest_error!("(request_flight) {} {:?}", &error_msg, window.timestamp_max);
            return Err(StatusCode::BAD_REQUEST);
        }

        flight_query.latest_arrival_time = Some(window.timestamp_max.into());
    }

    if let Some(window) = payload.time_depart_window {
        if window.timestamp_max <= current_time {
            let error_msg = "max depart time is in the past.".to_string();
            rest_error!("(request_flight) {} {:?}", &error_msg, window.timestamp_max);
            return Err(StatusCode::BAD_REQUEST);
        }

        let delta = Duration::try_minutes(ADVANCE_NOTICE_MINUTES).ok_or_else(|| {
            rest_error!("(request_flight) could not get time delta.");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        if window.timestamp_min <= (current_time + delta) {
            rest_error!("(request_flight) minimum departure window needs less than {ADVANCE_NOTICE_MINUTES} from now.");
            return Err(StatusCode::BAD_REQUEST);
        }

        flight_query.earliest_departure_time = Some(window.timestamp_min.into());
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
    let result = grpc_clients
        .scheduler
        .query_flight(flight_query)
        .await
        .map_err(|e| {
            rest_error!("(request_flight) svc-scheduler error {:?}", e);

            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_inner()
        .itineraries;

    Ok(result)
}

/// Unpacks flight plans from the scheduler into a format that
///  can be returned to the customer
fn unpack_itineraries(mut itineraries: Vec<SchedulerItinerary>) -> Vec<Itinerary> {
    let mut unpacked = vec![];

    for itinerary in itineraries.iter_mut() {
        let Ok(flight_plans) = itinerary
            .flight_plans
            .clone()
            .into_iter()
            .map(|fp| fp.try_into())
            .collect::<Result<Vec<FlightPlan>, _>>()
        else {
            rest_error!(
                "(request_flight) invalid flight plans in itinerary: {:?}",
                itinerary
            );
            continue;
        };

        unpacked.push(Itinerary {
            flight_plans,
            invoice: vec![],
            currency_unit: CurrencyUnit::Euro,
            ..Default::default()
        });
    }

    unpacked
}

/// Get the price for each itinerary
async fn update_pricing(
    payload: &QueryItineraryRequest,
    itinerary: &mut Itinerary,
    grpc_clients: &mut GrpcClients,
) -> Result<(), StatusCode> {
    let requests = itinerary
        .flight_plans
        .iter()
        .filter_map(|flight_plan| {
            let mut weight_g: u32 = 0;

            // add parcel weight
            if flight_plan.origin_vertiport_id == payload.origin_vertiport_id
                || flight_plan.target_vertiport_id == payload.target_vertiport_id
            {
                weight_g = payload.cargo_weight_g;
            }

            let distance_meters = match get_distance_meters(&flight_plan.path) {
                Ok(d) => d,
                Err(e) => {
                    rest_error!("(request_flight) invalid flight plan path: {:?}", e);
                    return None;
                }
            };

            Some(pricing::PricingRequest {
                service_type: pricing::pricing_request::ServiceType::Cargo as i32,
                distance_km: distance_meters / 1000.0,
                weight_kg: (weight_g as f32) / 1000.0,
            })
        })
        .collect::<Vec<pricing::PricingRequest>>();

    // At least one flight plan should have weight
    if requests.iter().all(|r| r.weight_kg == 0.0) {
        rest_error!("(request_flight) no flight plans with weight.");
        rest_debug!("(request_flight) query payload: {:?}", &payload);
        rest_debug!("(request_flight) itinerary: {:?}", &itinerary);

        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    if requests.len() != itinerary.flight_plans.len() {
        rest_error!("(request_flight) invalid pricing request count.");
        rest_debug!("(request_flight) query payload: {:?}", &payload);
        rest_debug!("(request_flight) itinerary: {:?}", &itinerary);

        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Make request, process response
    let bill = grpc_clients
        .pricing
        .get_pricing(pricing::PricingRequests { requests })
        .await
        .map_err(|e| {
            rest_error!("(request_flight) svc-pricing error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_inner();

    let mut names: HashMap<String, String> = HashMap::new();
    for flight_plan in itinerary.flight_plans.iter() {
        if !names.contains_key(&flight_plan.origin_vertiport_id) {
            let Ok(object) = grpc_clients
                .storage
                .vertiport
                .get_by_id(Id {
                    id: flight_plan.origin_vertiport_id.clone(),
                })
                .await
            else {
                continue;
            };

            let Some(data) = object.into_inner().data else {
                continue;
            };

            names.insert(flight_plan.origin_vertiport_id.clone(), data.name);
        }

        if !names.contains_key(&flight_plan.target_vertiport_id) {
            let Ok(object) = grpc_clients
                .storage
                .vertiport
                .get_by_id(Id {
                    id: flight_plan.target_vertiport_id.clone(),
                })
                .await
            else {
                continue;
            };

            let Some(data) = object.into_inner().data else {
                continue;
            };

            names.insert(flight_plan.target_vertiport_id.clone(), data.name);
        }
    }

    itinerary.invoice = bill
        .prices
        .iter()
        .zip(itinerary.flight_plans.iter_mut())
        .map(|(price, plan)| {
            let origin_vertiport_name = names
                .get(&plan.origin_vertiport_id)
                .unwrap_or(&plan.origin_vertiport_id)
                .clone();

            let target_vertiport_name = names
                .get(&plan.target_vertiport_id)
                .unwrap_or(&plan.target_vertiport_id)
                .clone();

            InvoiceItem {
                item: format!("\"{origin_vertiport_name}\" => \"{target_vertiport_name}\"",),
                cost: *price,
            }
        })
        .collect();

    Ok(())
}

/// Get Available Flights
///
/// Search for available trips and return a list of [`Itinerary`].
#[utoipa::path(
    post,
    path = "/cargo/request",
    tag = "svc-cargo",
    request_body = QueryItineraryRequest,
    responses(
        (status = 200, description = "List available flight plans", body = [Itinerary]),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "svc-scheduler or svc-pricing returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    )
)]
pub async fn request_flight(
    Extension(mut grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<QueryItineraryRequest>,
) -> Result<Json<Vec<DraftItinerary>>, StatusCode> {
    rest_debug!("(request_flight) entry.");

    //
    // Validate Request
    validate_payload(&payload)?;

    //
    // Query Flight with Scheduler
    let itineraries = scheduler_query(&payload, &mut grpc_clients).await?;

    //
    // Unpack flight itineraries
    let mut itineraries: Vec<Itinerary> = unpack_itineraries(itineraries);

    //
    // Get pricing for each itinerary
    for itinerary in itineraries.iter_mut() {
        itinerary.acquisition_vertiport_id = payload.origin_vertiport_id.clone();
        itinerary.delivery_vertiport_id = payload.target_vertiport_id.clone();
        itinerary.user_id = payload.user_id.clone();
        itinerary.cargo_weight_g = payload.cargo_weight_g;
        update_pricing(&payload, itinerary, &mut grpc_clients).await?;
    }

    //
    // Write all itineraries to redis
    let Some(mut pool) = crate::cache::pool::get_pool().await else {
        rest_error!("(store_itinerary) Couldn't get the redis pool.");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let mut draft_itineraries: Vec<DraftItinerary> = vec![];
    for itinerary in itineraries.into_iter() {
        let draft_id = match pool.store_itinerary(&itinerary).await {
            Ok(draft_id) => draft_id,
            Err(e) => {
                rest_warn!(
                    "(request_flight) error storing itinerary: {:?}; {}",
                    itinerary,
                    e
                );
                continue;
            }
        };

        draft_itineraries.push(DraftItinerary {
            id: draft_id.to_string(),
            itinerary,
        });
    }

    rest_debug!(
        "(request_flight) exit with {} itineraries.",
        draft_itineraries.len()
    );

    Ok(Json(draft_itineraries))
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
        let mut data = flight_plan::mock::get_data_obj();
        data.path = Some(GeoLineString {
            points: vec![
                GeoPoint {
                    latitude: 52.37488619450752,
                    longitude: 4.916048576268328,
                    altitude: 10.0,
                },
                GeoPoint {
                    latitude: 52.37488619450752,
                    longitude: 4.916048576268328,
                    altitude: 10.0,
                },
            ],
        });
        data.origin_timeslot_start = Some(Utc::now().into());
        data.origin_timeslot_end = Some((Utc::now() + Duration::try_minutes(10).unwrap()).into());
        data.target_timeslot_start = Some((Utc::now() + Duration::try_hours(1).unwrap()).into());
        data.target_timeslot_start = Some(
            (Utc::now() + Duration::try_hours(1).unwrap() + Duration::try_minutes(10).unwrap())
                .into(),
        );

        let flight_plan = flight_plan::Object {
            id: Uuid::new_v4().to_string(),
            data: Some(data.clone()),
        };

        let result_data = data.clone();
        let flight_plan_data = flight_plan.data.unwrap();
        assert_eq!(
            result_data.origin_vertiport_id.unwrap(),
            flight_plan_data.origin_vertiport_id.unwrap()
        );
        assert_eq!(
            result_data.target_vertiport_id.unwrap(),
            flight_plan_data.target_vertiport_id.unwrap()
        );
        assert_eq!(
            result_data.path.unwrap().points,
            flight_plan_data.path.unwrap().points
        );
    }
}
