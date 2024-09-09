pub use super::rest_types::{
    DraftItinerary, FlightPlan, InvoiceItem, Itinerary, QueryItineraryRequest,
};
use crate::grpc::client::GrpcClients;
use axum::{extract::Extension, Json};
use hyper::StatusCode;
use lib_common::time::{DateTime, Duration, Utc};
use lib_common::uuid::{to_uuid, Uuid};
use std::fmt::{self, Display, Formatter};

//
// Other Service Dependencies
//
use crate::cache::pool::ItineraryPool;
use crate::rest::rest_types::{CurrencyUnit, TimeWindow};
use std::collections::HashMap;
use svc_pricing_client_grpc::prelude::*;
use svc_scheduler_client_grpc::client::Itinerary as SchedulerItinerary;
use svc_scheduler_client_grpc::client::QueryFlightRequest;
use svc_scheduler_client_grpc::prelude::FlightPriority;
use svc_scheduler_client_grpc::prelude::SchedulerServiceClient;
use svc_storage_client_grpc::prelude::Id;
use svc_storage_client_grpc::simple_service::Client;

/// Don't allow excessively heavy loads
const MAX_CARGO_WEIGHT_G: u32 = 1_000_000; // 1000 kg

/// Advance notice required
const ADVANCE_NOTICE_MINUTES: i64 = 5;

/// Max window to search within
/// TODO(R5): for the demo
const MAX_TIME_WINDOW_HOURS: i64 = 8;

#[derive(Debug, PartialEq)]
enum ValidationError {
    /// The weight is too high or too low
    Weight,

    /// The minimum time is invalid
    TimeWindowMin,

    /// The maximum time is invalid
    TimeWindowMax,

    /// The origin vertiport ID is invalid
    OriginVertiportId,

    /// The target vertiport ID is invalid
    TargetVertiportId,

    /// The user ID is invalid
    UserId,

    /// Couldn't get a time delta
    BadTimeDelta,
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ValidationError::Weight => write!(f, "invalid weight"),
            ValidationError::TimeWindowMin => write!(f, "invalid time window min"),
            ValidationError::TimeWindowMax => write!(f, "invalid time window max"),
            ValidationError::OriginVertiportId => write!(f, "invalid origin vertiport id"),
            ValidationError::TargetVertiportId => write!(f, "invalid target vertiport id"),
            ValidationError::UserId => write!(f, "invalid user id"),
            ValidationError::BadTimeDelta => write!(f, "could not get time delta"),
        }
    }
}

/// Confirms that a payload has valid fields
fn validate_payload(payload: &QueryItineraryRequest) -> Result<(), ValidationError> {
    // Reject extreme weights
    if payload.cargo_weight_g > MAX_CARGO_WEIGHT_G {
        let error_msg = format!("request cargo weight exceeds {MAX_CARGO_WEIGHT_G}.");
        rest_error!("{}", &error_msg);
        return Err(ValidationError::Weight);
    }

    let time_window: &TimeWindow = &payload.time_depart_window;
    if time_window.timestamp_min >= time_window.timestamp_max {
        rest_error!("invalid departure time window.");
        return Err(ValidationError::TimeWindowMax);
    }

    let current_time = Utc::now();
    if time_window.timestamp_max <= current_time {
        rest_error!(
            "max depart time is in the past: {:?}",
            time_window.timestamp_max
        );

        return Err(ValidationError::TimeWindowMax);
    }

    #[cfg(not(tarpaulin_include))]
    // no_coverage: (R5) will never fail
    let delta = Duration::try_minutes(ADVANCE_NOTICE_MINUTES).ok_or_else(|| {
        rest_error!("could not get time delta.");
        ValidationError::BadTimeDelta
    })?;

    if time_window.timestamp_min <= (current_time + delta) {
        rest_error!("minimum departure window needs less than {ADVANCE_NOTICE_MINUTES} from now.");
        return Err(ValidationError::TimeWindowMin);
    }

    to_uuid(&payload.origin_vertiport_id).ok_or_else(|| {
        rest_error!("origin port ID not UUID format.");
        ValidationError::OriginVertiportId
    })?;

    to_uuid(&payload.target_vertiport_id).ok_or_else(|| {
        rest_error!("target port ID not UUID format.");
        ValidationError::TargetVertiportId
    })?;

    to_uuid(&payload.user_id).ok_or_else(|| {
        rest_error!("user ID not UUID format.");
        ValidationError::UserId
    })?;

    Ok(())
}

/// Query the scheduler for flight plans
async fn scheduler_query(
    payload: &QueryItineraryRequest,
    grpc_clients: &mut GrpcClients,
) -> Result<Vec<SchedulerItinerary>, StatusCode> {
    //
    // Validate Request
    validate_payload(payload).map_err(|e| {
        rest_error!("invalid request: {:?}", e);
        StatusCode::BAD_REQUEST
    })?;

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

    flight_query.earliest_departure_time = Some(payload.time_depart_window.timestamp_min.into());

    #[cfg(not(tarpaulin_include))]
    // no_coverage: (R5) will never fail
    let delta = Duration::try_hours(MAX_TIME_WINDOW_HOURS).ok_or_else(|| {
        rest_error!("could not get time delta.");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let latest: DateTime<Utc> = Utc::now() + delta;
    flight_query.latest_arrival_time = Some(latest.into());

    //
    // GRPC Request
    //
    #[cfg(not(tarpaulin_include))]
    // no_coverage: (R5) need backends to test (integration)
    let result = grpc_clients
        .scheduler
        .query_flight(flight_query)
        .await
        .map_err(|e| {
            rest_error!("svc-scheduler error {:?}", e);

            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_inner()
        .itineraries;

    Ok(result)
}

/// Unpacks flight plans from the scheduler into a format that
///  can be returned to the customer
fn unpack_itineraries(itineraries: Vec<SchedulerItinerary>) -> Vec<Itinerary> {
    itineraries
        .into_iter()
        .filter_map(|itinerary| {
            itinerary
                .flight_plans
                .into_iter()
                .map(|fp| fp.try_into())
                .collect::<Result<Vec<FlightPlan>, _>>()
                .map_err(|_| {
                    rest_error!("invalid flight plans in itinerary, skipping.",);
                })
                .map(|plans| Itinerary {
                    flight_plans: plans,
                    invoice: vec![],
                    currency_unit: CurrencyUnit::Euro,
                    ..Default::default()
                })
                .ok()
        })
        .collect::<Vec<Itinerary>>()
}

/// Get the price for each itinerary
#[cfg(not(tarpaulin_include))]
// no_coverage: (R5) function test not yet created
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

            let Some(distance_meters) = super::utils::get_distance_meters(&flight_plan.path) else {
                rest_error!("invalid flight plan path.");
                return None;
            };

            Some(pricing::PricingRequest {
                service_type: pricing::pricing_request::ServiceType::Cargo as i32,
                distance_km: (distance_meters / 1000.0) as f32,
                weight_kg: (weight_g as f32) / 1000.0,
            })
        })
        .collect::<Vec<pricing::PricingRequest>>();

    // At least one flight plan should have weight
    if requests.iter().all(|r| r.weight_kg == 0.0) {
        rest_error!("no flight plans with weight.");
        rest_debug!("query payload: {:?}", &payload);
        rest_debug!("itinerary: {:?}", &itinerary);

        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    if requests.len() != itinerary.flight_plans.len() {
        rest_error!("invalid pricing request count.");
        rest_debug!("query payload: {:?}", &payload);
        rest_debug!("itinerary: {:?}", &itinerary);

        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Make request, process response
    let bill = grpc_clients
        .pricing
        .get_pricing(pricing::PricingRequests { requests })
        .await
        .map_err(|e| {
            rest_error!("svc-pricing error: {e}");
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
#[cfg(not(tarpaulin_include))]
// no_coverage: (R5) function test not yet created
pub async fn request_flight(
    Extension(mut grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<QueryItineraryRequest>,
) -> Result<Json<Vec<DraftItinerary>>, StatusCode> {
    rest_debug!("entry.");
    //
    // Query Flight with Scheduler
    let itineraries = scheduler_query(&payload, &mut grpc_clients).await?;

    //
    // Unpack flight itineraries
    let mut itineraries: Vec<Itinerary> = unpack_itineraries(itineraries);

    //
    // Get pricing for each itinerary
    for itinerary in itineraries.iter_mut() {
        itinerary
            .acquisition_vertiport_id
            .clone_from(&payload.origin_vertiport_id);
        itinerary
            .delivery_vertiport_id
            .clone_from(&payload.target_vertiport_id);
        itinerary.user_id.clone_from(&payload.user_id);
        itinerary.cargo_weight_g = payload.cargo_weight_g;
        update_pricing(&payload, itinerary, &mut grpc_clients).await?;
    }

    //
    // Write all itineraries to redis
    let arc = crate::cache::pool::get_pool().await.map_err(|e| {
        rest_error!("Couldn't get the redis pool: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut pool = arc.lock().await;
    let mut draft_itineraries: Vec<DraftItinerary> = vec![];
    for itinerary in itineraries.into_iter() {
        let itinerary_id = Uuid::new_v4().to_string();

        if let Err(e) = (*pool)
            .store_itinerary(itinerary_id.clone(), &itinerary)
            .await
        {
            rest_warn!("error storing itinerary: {:?}; {}", itinerary, e);

            continue;
        }

        draft_itineraries.push(DraftItinerary {
            id: itinerary_id,
            itinerary,
        });
    }

    rest_debug!("exit with {} itineraries.", draft_itineraries.len());

    Ok(Json(draft_itineraries))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_common::time::{Duration, Utc};
    use lib_common::uuid::Uuid;
    use svc_scheduler_client_grpc::prelude::scheduler_storage::flight_plan;
    use svc_scheduler_client_grpc::prelude::scheduler_storage::{GeoLineStringZ, GeoPointZ};

    #[test]
    fn test_time_constants() {
        // test that unwrapping on these values will succeed, so
        // we can hide these calls from coverage when they're in the
        // middle of a function (untestable)
        Duration::try_minutes(ADVANCE_NOTICE_MINUTES).unwrap();
        Duration::try_hours(MAX_TIME_WINDOW_HOURS).unwrap();
    }

    #[test]
    fn test_flight_plan_object_to_return_type() {
        let mut data = flight_plan::mock::get_data_obj();
        data.path = Some(GeoLineStringZ {
            points: vec![
                GeoPointZ {
                    x: 52.37488619450752,
                    y: 4.916048576268328,
                    z: 10.0,
                },
                GeoPointZ {
                    x: 52.37488619450752,
                    y: 4.916048576268328,
                    z: 10.0,
                },
            ],
        });
        data.origin_vertiport_id = Some(lib_common::uuid::Uuid::new_v4().to_string());
        data.target_vertiport_id = Some(lib_common::uuid::Uuid::new_v4().to_string());
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

    #[test]
    fn test_unpack_itineraries() {
        let mut data = flight_plan::mock::get_data_obj();
        // add vertiport ids since they're required here but not provided by the mock service since
        // they are optional and are being deducted from the vertipads
        data.origin_vertiport_id = Some(lib_common::uuid::Uuid::new_v4().to_string());
        data.target_vertiport_id = Some(lib_common::uuid::Uuid::new_v4().to_string());

        let itineraries = vec![
            SchedulerItinerary {
                flight_plans: vec![data.clone()],
            },
            SchedulerItinerary {
                flight_plans: vec![data.clone()],
            },
        ];

        let result = unpack_itineraries(itineraries);
        assert_eq!(result.len(), 2);

        // some invalid flight plans
        let itineraries = vec![
            SchedulerItinerary {
                flight_plans: vec![flight_plan::Data {
                    origin_vertiport_id: Some("invalid".to_string()),
                    ..data.clone()
                }],
            },
            SchedulerItinerary {
                flight_plans: vec![data.clone()],
            },
        ];

        let result = unpack_itineraries(itineraries);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_validate_payload() {
        let mut payload = QueryItineraryRequest {
            cargo_weight_g: MAX_CARGO_WEIGHT_G,
            time_depart_window: TimeWindow {
                timestamp_min: Utc::now()
                    + Duration::try_minutes(ADVANCE_NOTICE_MINUTES + 1).unwrap(),
                timestamp_max: Utc::now() + Duration::try_minutes(10).unwrap(),
            },
            target_vertiport_id: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4().to_string(),
            origin_vertiport_id: Uuid::new_v4().to_string(),
        };

        validate_payload(&payload).unwrap();

        // bad weight
        payload.cargo_weight_g = MAX_CARGO_WEIGHT_G + 1;
        assert_eq!(validate_payload(&payload), Err(ValidationError::Weight));
        payload.cargo_weight_g = MAX_CARGO_WEIGHT_G;

        // min time too soon
        payload.time_depart_window.timestamp_min =
            Utc::now() + Duration::try_minutes(ADVANCE_NOTICE_MINUTES - 1).unwrap();
        assert_eq!(
            validate_payload(&payload),
            Err(ValidationError::TimeWindowMin)
        );
        payload.time_depart_window.timestamp_min =
            Utc::now() + Duration::try_minutes(ADVANCE_NOTICE_MINUTES + 1).unwrap();

        // bad max time (<= min time)
        payload.time_depart_window.timestamp_max = payload.time_depart_window.timestamp_min; // same as min
        assert_eq!(
            validate_payload(&payload),
            Err(ValidationError::TimeWindowMax)
        );
        payload.time_depart_window.timestamp_max =
            payload.time_depart_window.timestamp_min - Duration::try_milliseconds(1).unwrap(); // less than min
        assert_eq!(
            validate_payload(&payload),
            Err(ValidationError::TimeWindowMax)
        );

        // max time in the past
        payload.time_depart_window.timestamp_max = Utc::now() - Duration::try_seconds(10).unwrap();
        payload.time_depart_window.timestamp_min =
            payload.time_depart_window.timestamp_max - Duration::try_seconds(10).unwrap();
        assert_eq!(
            validate_payload(&payload),
            Err(ValidationError::TimeWindowMax)
        );

        payload.time_depart_window.timestamp_min =
            Utc::now() + Duration::try_minutes(ADVANCE_NOTICE_MINUTES + 1).unwrap();
        payload.time_depart_window.timestamp_max =
            payload.time_depart_window.timestamp_min + Duration::try_minutes(10).unwrap();

        // bad target vertiport id
        payload.target_vertiport_id = "not a uuid".to_string();
        assert_eq!(
            validate_payload(&payload),
            Err(ValidationError::TargetVertiportId)
        );
        payload.target_vertiport_id = Uuid::new_v4().to_string();

        // bad user id
        payload.user_id = "not a uuid".to_string();
        assert_eq!(validate_payload(&payload), Err(ValidationError::UserId));
        payload.user_id = Uuid::new_v4().to_string();

        // bad origin vertiport id
        payload.origin_vertiport_id = "not a uuid".to_string();
        assert_eq!(
            validate_payload(&payload),
            Err(ValidationError::OriginVertiportId)
        );
        payload.origin_vertiport_id = Uuid::new_v4().to_string();
    }

    #[tokio::test]
    async fn test_scheduler_query() {
        let config = crate::config::Config::default();
        let mut grpc_clients = GrpcClients::default(config);

        let mut payload = QueryItineraryRequest {
            cargo_weight_g: 100,
            time_depart_window: TimeWindow {
                timestamp_min: Utc::now()
                    + Duration::try_minutes(ADVANCE_NOTICE_MINUTES + 1).unwrap(),
                timestamp_max: Utc::now() + Duration::try_minutes(10).unwrap(),
            },
            target_vertiport_id: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4().to_string(),
            origin_vertiport_id: Uuid::new_v4().to_string(),
        };

        scheduler_query(&payload, &mut grpc_clients).await.unwrap();

        // full request validation tested in another UT, so we can just test one error case here
        payload.cargo_weight_g = MAX_CARGO_WEIGHT_G + 1;
        assert_eq!(
            scheduler_query(&payload, &mut grpc_clients)
                .await
                .unwrap_err(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn test_validation_error_display() {
        assert_eq!(
            ValidationError::Weight.to_string(),
            "invalid weight".to_string()
        );
        assert_eq!(
            ValidationError::TimeWindowMin.to_string(),
            "invalid time window min".to_string()
        );
        assert_eq!(
            ValidationError::TimeWindowMax.to_string(),
            "invalid time window max".to_string()
        );
        assert_eq!(
            ValidationError::OriginVertiportId.to_string(),
            "invalid origin vertiport id".to_string()
        );
        assert_eq!(
            ValidationError::TargetVertiportId.to_string(),
            "invalid target vertiport id".to_string()
        );
        assert_eq!(
            ValidationError::UserId.to_string(),
            "invalid user id".to_string()
        );
        assert_eq!(
            ValidationError::BadTimeDelta.to_string(),
            "could not get time delta".to_string()
        );
    }
}
