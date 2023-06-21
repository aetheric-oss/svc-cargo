pub mod rest_types {
    include!("../../../openapi/types.rs");
}

use axum::{extract::Extension, Json};
use chrono::Utc;
use hyper::StatusCode;
use lib_common::time::{datetime_to_timestamp, timestamp_to_datetime};
use uuid::Uuid;

use crate::grpc::client::GrpcClients;
use svc_storage_client_grpc::resources::itinerary::ItineraryStatus as StorageItineraryStatus;
use svc_storage_client_grpc::resources::{
    parcel::Data as ParcelData, parcel::ParcelStatus, parcel_scan::Data as ParcelScanData, GeoPoint,
};
use svc_storage_client_grpc::ClientConnect;
use svc_storage_client_grpc::{AdvancedSearchFilter, Id};

use svc_pricing_client::pricing_grpc::{
    pricing_request::ServiceType, PricingRequest, PricingRequests,
};

use svc_scheduler_client_grpc::grpc::{
    ConfirmItineraryRequest, Id as ResourceId, Itinerary as SchedulerItinerary, QueryFlightPlan,
    QueryFlightRequest,
};

pub use rest_types::{
    FlightLeg, FlightQuery, Itinerary, ItineraryCancel, ItineraryConfirm, ItineraryConfirmation,
    ParcelScan, Vertiport, VertiportsQuery,
};

pub use rest_types::{ItineraryInfo, ItineraryInfoList, ItineraryStatus};

/// Don't allow excessively heavy loads
const MAX_CARGO_WEIGHT_G: u32 = 1_000_000; // 1000 kg

/// Don't allow large UUID strings
const UUID_MAX_SIZE: usize = 50; // Sometimes braces or hyphens

/// Returns true if a given string is UUID format
fn is_uuid(s: &str) -> bool {
    // Prevent buffer overflows
    if s.len() > UUID_MAX_SIZE {
        rest_error!("(is_uuid) input string larger than expected: {}.", s.len());
        return false;
    }

    Uuid::parse_str(s).is_ok()
}

/// Parses the incoming flight plans for information the customer wants
fn parse_flight(plan: &QueryFlightPlan) -> Option<FlightLeg> {
    let Some(prost_time) = plan.estimated_departure.clone() else {
        let error_msg = "no departure time in flight plan; discarding.";
        rest_error!("(parse_flight) {}", &error_msg);
        return None;
    };

    let Some(time_depart) = timestamp_to_datetime(&prost_time) else {
        let error_msg = "can't convert prost timestamp to datetime.";
        rest_error!("(parse_flight) {}", &error_msg);
        return None;
    };

    let Some(prost_time) = plan.estimated_arrival.clone() else {
        let error_msg = "(parse_flight) no arrival time in flight plan; discarding.";
        rest_error!("(parse_flight) {}", &error_msg);
        return None;
    };

    let Some(time_arrive) = timestamp_to_datetime(&prost_time) else {
        let error_msg = "can't convert prost timestamp to datetime.";
        rest_error!("(parse_flight) {}", &error_msg);
        return None;
    };

    Some(FlightLeg {
        flight_plan_id: plan.id.clone(),
        vertiport_depart_id: plan.vertiport_depart_id.to_string(),
        vertiport_arrive_id: plan.vertiport_arrive_id.to_string(),
        timestamp_depart: time_depart,
        timestamp_arrive: time_arrive,
        distance_m: plan.estimated_distance as f32,
        base_pricing: None,
        currency_type: None,
    })
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "svc-cargo",
    responses(
        (status = 200, description = "Service is healthy, all dependencies running."),
        (status = 503, description = "Service is unhealthy, one or more dependencies unavailable.")
    )
)]
pub async fn health_check(
    Extension(mut grpc_clients): Extension<GrpcClients>,
) -> Result<(), StatusCode> {
    rest_debug!("(health_check) entry.");

    let mut ok = true;

    if grpc_clients.storage.vertiport.get_client().await.is_err() {
        let error_msg = "svc-storage vertiport client unavailable.".to_string();
        rest_error!("(health_check) {}", &error_msg);
        ok = false;
    };

    if grpc_clients.storage.parcel.get_client().await.is_err() {
        let error_msg = "svc-storage parcel client unavailable.".to_string();
        rest_error!("(health_check) {}", &error_msg);
        ok = false;
    };

    if grpc_clients.storage.parcel_scan.get_client().await.is_err() {
        let error_msg = "svc-storage parcel scan client unavailable.".to_string();
        rest_error!("(health_check) {}", &error_msg);
        ok = false;
    };

    let result = grpc_clients.pricing.get_client().await;
    if result.is_none() {
        let error_msg = "svc-pricing unavailable.".to_string();
        rest_error!("(health_check) {}", &error_msg);
        ok = false;
    };

    let result = grpc_clients.scheduler.get_client().await;
    if result.is_none() {
        let error_msg = "svc-scheduler unavailable.".to_string();
        rest_error!("(health_check) {}", &error_msg);
        ok = false;
    };

    match ok {
        true => {
            rest_info!("(health_check) healthy, all dependencies running.");
            Ok(())
        }
        false => {
            rest_error!("(health_check) unhealthy, 1+ dependencies down.");
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
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
    Extension(grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<VertiportsQuery>,
) -> Result<Json<Vec<Vertiport>>, StatusCode> {
    rest_debug!("(query_vertiports) entry.");

    //
    // 1 degree of latitude ~= 69 miles
    // 1 degree of longitude ~= 55 miles
    //
    // TODO(R3) This may be commanded by the GUI, if someone is scrolled out
    //  far on the map the degree_range should increase
    let degree_range: f32 = 2.0;
    let filter = AdvancedSearchFilter::search_between(
        "latitude".to_owned(),
        (payload.latitude + degree_range).to_string(),
        (payload.latitude - degree_range).to_string(),
    )
    .and_between(
        "longitude".to_owned(),
        (payload.longitude + degree_range).to_string(),
        (payload.longitude - degree_range).to_string(),
    );
    let request = tonic::Request::new(filter);

    // Get Client
    let Ok(mut client) = grpc_clients.storage.vertiport.get_client().await else {
        let error_msg = "svc-storage unavailable.".to_string();
        rest_error!("(query_vertiports) {}", &error_msg);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    // Make request, process response
    let Ok(response) = client.search(request).await else {
        let error_msg = "error response from svc-storage.".to_string();
        rest_error!("(query_vertiports) {}.", &error_msg);
        return Err(StatusCode::INTERNAL_SERVER_ERROR)
    };

    let mut vertiports: Vec<Vertiport> = vec![];
    for obj in response.into_inner().list {
        let Some(data) = obj.data else {
            rest_error!("(query_vertiports) vertiport data is None.");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        let Some(location) = data.geo_location else {
            rest_error!("(query_vertiports) vertiport location is None.");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        let Some(exterior) = location.exterior else {
            rest_error!("(query_vertiports) vertiport exterior is None.");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        let points = exterior.points;
        let latitude = points.iter().map(|pt| pt.x).sum::<f64>() / points.len() as f64;
        let longitude = points.iter().map(|pt| pt.y).sum::<f64>() / points.len() as f64;

        vertiports.push(Vertiport {
            id: obj.id,
            label: data.description,
            latitude: latitude as f32,
            longitude: longitude as f32,
        })
    }

    rest_info!("(query_vertiports) found {} vertiports.", vertiports.len());
    Ok(Json(vertiports))
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
) -> Result<Json<Vec<Itinerary>>, StatusCode> {
    rest_debug!("(query_flight) entry.");

    //
    // Validate Request
    //

    // Reject extreme weights
    let weight_g: u32 = (payload.cargo_weight_kg * 1000.0) as u32;
    if weight_g >= MAX_CARGO_WEIGHT_G {
        let error_msg = format!("request cargo weight exceeds {MAX_CARGO_WEIGHT_G}.");
        rest_error!("(query_flight) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check UUID validity
    if !is_uuid(&payload.vertiport_arrive_id) {
        let error_msg = "arrival port ID not UUID format.".to_string();
        rest_error!("(query_flight) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    if !is_uuid(&payload.vertiport_depart_id) {
        let error_msg = "departure port ID not UUID format.".to_string();
        rest_error!("(query_flight) {}", &error_msg);
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
            rest_error!("(query_flight) {} {:?}", &error_msg, window.timestamp_max);
            return Err(StatusCode::BAD_REQUEST);
        }

        let Some(ts) = datetime_to_timestamp(&window.timestamp_max) else {
            let error_msg = "unable to convert datetime to timestamp.".to_string();
            rest_error!("(query_flight) {} {:?}", &error_msg, window.timestamp_max);
            return Err(StatusCode::BAD_REQUEST);
        };

        flight_query.latest_arrival_time = Some(ts);
    }

    if let Some(window) = payload.time_depart_window {
        if window.timestamp_max <= current_time {
            let error_msg = "max depart time is in the past.".to_string();
            rest_error!("(query_flight) {} {:?}", &error_msg, window.timestamp_max);
            return Err(StatusCode::BAD_REQUEST);
        }

        let Some(ts) = datetime_to_timestamp(&window.timestamp_max) else {
            let error_msg = "unable to convert datetime to timestamp.".to_string();
            rest_error!("(query_flight) {} {:?}", &error_msg, window.timestamp_max);
            return Err(StatusCode::BAD_REQUEST);
        };

        flight_query.earliest_departure_time = Some(ts);
    }

    if flight_query.earliest_departure_time.is_none() && flight_query.latest_arrival_time.is_none()
    {
        let error_msg = "invalid time window.".to_string();
        rest_error!("(query_flight) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    //
    // GRPC Request
    //

    let Some(mut scheduler_client) = grpc_clients.scheduler.get_client().await else {
        let error_msg = "svc-scheduler unavailable.".to_string();
        rest_error!("(query_flight) {}", &error_msg);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let Some(mut pricing_client) = grpc_clients.pricing.get_client().await else {
        let error_msg = "svc-pricing unavailable.".to_string();
        rest_error!("(query_flight) {}", &error_msg);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let request = tonic::Request::new(flight_query);
    let response = scheduler_client.query_flight(request).await;
    let Ok(response) = response else {
        let error_msg = "svc-scheduler error.".to_string();
        rest_error!("(query_flight) {} {:?}", &error_msg, response.unwrap_err());
        rest_error!("(query_flight) invalidating svc-scheduler client.");
        grpc_clients.scheduler.invalidate().await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let itineraries: Vec<SchedulerItinerary> = response.into_inner().itineraries;

    //
    // Unpack flight itineraries
    //

    // List of lists of flights
    let mut offerings: Vec<Itinerary> = vec![];
    for itinerary in &itineraries {
        let mut legs: Vec<FlightLeg> = vec![];

        if let Some(plan) = &itinerary.flight_plan {
            if let Some(leg) = parse_flight(plan) {
                legs.push(leg);
            }
        }

        for dh in &itinerary.deadhead_flight_plans {
            if let Some(leg) = parse_flight(dh) {
                legs.push(leg);
            }
        }

        offerings.push(Itinerary {
            id: itinerary.id.clone(),
            legs,
            base_pricing: None,
            currency_type: Some("usd".to_string()),
        })
    }
    rest_info!("(query_flight) found {} flight options.", offerings.len());

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
            rest_error!("(query_flight) {} {:?}", &error_msg, response.unwrap_err());
            rest_error!("(query_flight) invalidating svc-pricing client.");
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

    rest_debug!("(query_flight) exit with {} itineraries.", offerings.len());
    Ok(Json(offerings))
}

/// Confirm an itinerary
/// This will confirm an itinerary with the scheduler, and will register the parcel with
///  the storage service.
#[utoipa::path(
    put,
    path = "/cargo/confirm",
    tag = "svc-cargo",
    request_body = ItineraryConfirm,
    responses(
        (status = 200, description = "Itinerary confirmed", body = String),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "Microservice dependency returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    )
)]
pub async fn confirm_itinerary(
    Extension(mut grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<ItineraryConfirm>,
) -> Result<Json<ItineraryConfirmation>, StatusCode> {
    rest_debug!("(confirm_itinerary) entry.");

    if !is_uuid(&payload.id) {
        let error_msg = "flight plan ID not in UUID format.".to_string();
        rest_error!("(confirm_itinerary) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    //
    // Confirm itinerary with scheduler
    //
    let Some(mut client) = grpc_clients.scheduler.get_client().await else {
        let error_msg = "svc-scheduler unavailable.".to_string();
        rest_error!("(confirm_itinerary) {}", &error_msg);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    // Make request, process response
    let data = ConfirmItineraryRequest {
        id: payload.id,
        user_id: payload.user_id,
    };
    let request = tonic::Request::new(data);
    let response = match client.confirm_itinerary(request).await {
        Ok(response) => response.into_inner(),
        Err(e) => {
            let error_msg = "svc-scheduler error.".to_string();
            rest_error!("(confirm_itinerary) {} {:?}", &error_msg, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if !response.confirmed {
        let error_msg = "svc-scheduler confirm fail.".to_string();
        rest_error!("(confirm_itinerary) {}", &error_msg);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    //
    // Register Parcel with Storage
    //
    let itinerary_id = response.id;
    let data = ParcelData {
        itinerary_id: itinerary_id.clone(),
        status: ParcelStatus::Notdroppedoff as i32,
    };

    // TODO(R4): Push to queue, in case this call fails need a retry mechanism
    let request = tonic::Request::new(data);
    let Ok(mut client) = grpc_clients.storage.parcel.get_client().await else {
        let error_msg = "svc-parcel-storage unavailable.".to_string();
        rest_error!("(confirm_itinerary) {}", &error_msg);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    // Make request, process response
    let response = match client.insert(request).await {
        Ok(response) => response.into_inner(),
        Err(e) => {
            let error_msg = "svc-parcel-storage error.".to_string();
            rest_error!("(confirm_itinerary) {} {:?}", &error_msg, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let Some(result) = response.validation_result else {
        let error_msg = "svc-parcel-storage validation fail.".to_string();
        rest_error!("(confirm_itinerary) {}", &error_msg);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let Some(object) = response.object else {
        let error_msg = "svc-parcel-storage insert fail.".to_string();
        rest_error!("(confirm_itinerary) {}", &error_msg);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let parcel_id = object.id;
    if !result.success {
        let error_msg = "svc-parcel-storage insert fail.".to_string();
        rest_error!("(confirm_itinerary) {}", &error_msg);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json(ItineraryConfirmation {
        itinerary_id,
        parcel_id,
    }))
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
    request_body = ItineraryCancel
)]
pub async fn cancel_itinerary(
    Extension(mut grpc_clients): Extension<GrpcClients>,
    Json(payload): Json<ItineraryCancel>,
) -> Result<(), StatusCode> {
    rest_debug!("(cancel_itinerary) entry.");
    let itinerary_id = payload.id;
    if !is_uuid(&itinerary_id) {
        let error_msg = "itinerary ID not in UUID format.".to_string();
        rest_error!("(cancel_itinerary) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Get Client
    let client_option = grpc_clients.scheduler.get_client().await;
    let Some(mut client) = client_option else {
        let error_msg = "svc-scheduler unavailable.".to_string();
        rest_error!("(cancel_itinerary) {}", &error_msg);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    // Make request, process response
    let request = tonic::Request::new(ResourceId {
        id: itinerary_id.clone(),
    });
    let response = match client.cancel_itinerary(request).await {
        Ok(response) => response.into_inner(),
        Err(e) => {
            let error_msg = "svc-scheduler request fail.".to_string();
            rest_error!("(cancel_itinerary) {} {:?}", &error_msg, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if !response.cancelled {
        let error_msg = "svc-scheduler cancel fail.".to_string();
        rest_error!("(cancel_itinerary) {} {}", &error_msg, response.reason);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    rest_info!("(cancel_itinerary) successfully cancelled itinerary.");

    //
    // Get parcel from id
    //
    let Ok(mut client) = grpc_clients.storage.parcel.get_client().await else {
        let error_msg = "svc-parcel-storage unavailable.".to_string();
        rest_error!("(cancel_itinerary) {}", &error_msg);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let filter =
        AdvancedSearchFilter::search_equals("itinerary_id".to_string(), itinerary_id.clone());

    let list = match client.search(filter).await {
        Ok(response) => response.into_inner().list,
        Err(e) => {
            let error_msg = "svc-parcel-storage error.".to_string();
            rest_error!("(cancel_itinerary) {} {:?}", &error_msg, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // An itinerary might have multiple parcels
    // TODO(R4): Push these onto a queue in case any one fails
    let mut ok = true;
    for parcel in list {
        let request = tonic::Request::new(Id { id: parcel.id });
        match client.delete(request).await {
            Ok(_) => {
                // Delete activity currently returns Empty
                // response.into_inner()
            }
            Err(e) => {
                let error_msg = "svc-parcel-storage error.".to_string();
                rest_error!("(cancel_itinerary) {} {:?}", &error_msg, e);
                // Still try to delete other parcels
                ok = false;
            }
        };
    }

    if !ok {
        rest_error!("(cancel_itinerary) could not delete all parcels.");
    }

    // If the customer's itinerary was cancelled, but the parcels were not, it's still a success for them
    Ok(())
}

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
            x: payload.longitude,
            y: payload.latitude,
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

/// Scan a parcel
/// The provided parcel ID and scanner ID must already exist in the database
#[utoipa::path(
    put,
    path = "/cargo/status",
    tag = "svc-cargo",
    request_body = String,
    responses(
        (status = 200, description = "Status retrieved", body = String),
        (status = 400, description = "Request body is invalid format"),
        (status = 500, description = "svc-storage returned error"),
        (status = 503, description = "Could not connect to other microservice dependencies")
    )
)]
pub async fn query_itinerary_status(
    Extension(grpc_clients): Extension<GrpcClients>,
    Json(user_id): Json<String>,
) -> Result<Json<ItineraryInfoList>, StatusCode> {
    if !is_uuid(&user_id) {
        let error_msg = "itinerary ID not in UUID format.".to_string();
        rest_error!("(query_itinerary_status) {}", &error_msg);
        return Err(StatusCode::BAD_REQUEST);
    }

    //
    // Get Itinerary Client
    //
    let Ok(mut client) = grpc_clients.storage.itinerary.get_client().await else {
        let error_msg = "svc-storage unavailable.".to_string();
        rest_error!("(query_itinerary_status) {}", &error_msg);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let filter = AdvancedSearchFilter::search_equals("user_id".to_string(), user_id);
    // TODO(R4) incomplete itineraries only

    let request = tonic::Request::new(filter);
    let response = match client.search(request).await {
        Ok(response) => response.into_inner(),
        Err(e) => {
            let error_msg = "svc-storage error.".to_string();
            rest_error!("(query_itinerary_status) {} {:?}", &error_msg, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let Ok(mut client) = grpc_clients.storage.itinerary_flight_plan_link.get_client().await else {
        let error_msg = "svc-storage unavailable.".to_string();
        rest_error!("(query_itinerary_status) {}", &error_msg);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let mut itineraries: Vec<ItineraryInfo> = vec![];
    for itinerary in response.list {
        let itinerary_id = itinerary.id;
        let Some(data) = itinerary.data else {
            let error_msg = "svc-storage response invalid.".to_string();
            rest_error!("(query_itinerary_status) {}", &error_msg);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        // Get flight plans
        let request = tonic::Request::new(Id {
            id: itinerary_id.clone(),
        });

        let Ok(response) = client.get_linked(request).await else {
            let error_msg = "could not get linked flight plans from storage.".to_string();
            rest_error!("(query_itinerary_status) {}", &error_msg);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        let mut legs = vec![];
        for plan in response.into_inner().list {
            let Some(data) = plan.data else {
                let error_msg = "svc-storage response invalid.".to_string();
                rest_error!("(query_itinerary_status) {}", &error_msg);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            };

            legs.push(FlightLeg {
                flight_plan_id: plan.id.clone(),
                vertiport_depart_id: data.departure_vertipad_id,
                vertiport_arrive_id: data.destination_vertipad_id,
                timestamp_depart: timestamp_to_datetime(&data.scheduled_departure),
                timestamp_arrive: timestamp_to_datetime(&data.estimated_arrival),
                distance_m: data.estimated_distance,
                base_pricing: None,
                currency_type: None,
            });
        }

        let Some(status) = StorageItineraryStatus::from_i32(data.status) else {
            let error_msg = "svc-storage response invalid: bad itinerary status.".to_string();
            rest_error!("(query_itinerary_status) {}", &error_msg);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        let mut itinerary = ItineraryInfo {
            status: match status {
                StorageItineraryStatus::Cancelled => ItineraryStatus::Cancelled,
                StorageItineraryStatus::Active => ItineraryStatus::Active,
            },
            itinerary: Itinerary {
                id: itinerary_id,
                legs,
                base_pricing: None,
                currency_type: None,
            },
        };

        itineraries.push(itinerary)
    }

    Ok(())
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

        let Some(flight_plan) = parse_flight(&fp) else {
            panic!();
        };
        assert_eq!(fp.id, flight_plan.flight_plan_id);
        assert_eq!(fp.vertiport_depart_id, flight_plan.vertiport_depart_id);
        assert_eq!(fp.vertiport_arrive_id, flight_plan.vertiport_arrive_id);

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
