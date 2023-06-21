//! Example communication with this service

use chrono::{Duration, Utc};
use hyper::{Body, Client, Method, Request, Response};
use hyper::{Error, StatusCode};
use lib_common::grpc::get_endpoint_from_env;
use svc_cargo_client_rest::types::*;
use uuid::Uuid;

fn evaluate(resp: Result<Response<Body>, Error>, expected_code: StatusCode) -> (bool, String) {
    let mut ok = true;
    let result_str: String = match resp {
        Ok(r) => {
            let tmp = r.status() == expected_code;
            ok &= tmp;
            println!("{:?}", r.body());

            r.status().to_string()
        }
        Err(e) => {
            ok = false;
            e.to_string()
        }
    };

    (ok, result_str)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("NOTE: Ensure the server is running, or this example will fail.");

    let (host, port) = get_endpoint_from_env("SERVER_HOSTNAME", "SERVER_PORT_REST");
    let url = format!("http://{host}:{port}");
    let mut ok = true;
    let client = Client::builder()
        .pool_idle_timeout(std::time::Duration::from_secs(10))
        .build_http();

    // POST /cargo/vertiports
    {
        let data = VertiportsQuery {
            latitude: 52.37488619450752,
            longitude: 4.916048576268328,
        };

        let Ok(data_str) = serde_json::to_string(&data) else {
            panic!("Failed to serialize data");
        };

        let uri = format!("{}/cargo/vertiports", url);
        let Ok(req) = Request::builder()
            .method(Method::POST)
            .uri(uri.clone())
            .header("content-type", "application/json")
            .body(Body::from(data_str))
        else {
            panic!("Failed to build request");
        };

        let resp = client.request(req).await;
        let (success, result_str) = evaluate(resp, StatusCode::OK);
        ok &= success;

        println!("{}: {}", uri, result_str);
    }

    // POST /cargo/request
    {
        let depart_timestamp_min = Utc::now() + Duration::seconds(60);
        let data = FlightRequest {
            // Arbitrary UUIDs
            vertiport_depart_id: "cabcdd14-03ab-4ac0-b58c-dd4175bc587e".to_string(),
            vertiport_arrive_id: "59e51ad1-d57d-4d2c-bc2d-e2387367d17f".to_string(),
            time_depart_window: Some(TimeWindow {
                timestamp_min: depart_timestamp_min,
                timestamp_max: depart_timestamp_min + Duration::seconds(360),
            }),
            time_arrive_window: None,
            cargo_weight_kg: 1.0,
        };

        let Ok(data_str) = serde_json::to_string(&data) else {
            panic!("Failed to serialize data");
        };

        let uri = format!("{}/cargo/request", url);
        let Ok(req) = Request::builder()
            .method(Method::POST)
            .uri(uri.clone())
            .header("content-type", "application/json")
            .body(Body::from(data_str))
        else {
            panic!("Failed to build request");
        };

        let resp = client.request(req).await;
        let (success, result_str) = evaluate(resp, StatusCode::ACCEPTED);
        ok &= success;

        println!("{}: {}", uri, result_str);
    }

    // PUT /cargo/confirm
    {
        let data = ItineraryConfirm {
            // Arbitrary UUID
            id: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4().to_string(),
        };

        let Ok(data_str) = serde_json::to_string(&data) else {
            panic!("Failed to serialize data");
        };

        let uri = format!("{}/cargo/confirm", url);
        let Ok(req) = Request::builder()
            .method(Method::PUT)
            .uri(uri.clone())
            .header("content-type", "application/json")
            .body(Body::from(data_str))
        else {
            panic!("Failed to build request");
        };

        let resp = client.request(req).await;
        let (success, result_str) = evaluate(resp, StatusCode::OK);
        ok &= success;

        println!("{}: {}", uri, result_str);
    }

    // DELETE /cargo/cancel
    {
        let data = ItineraryCancel {
            // arbitrary UUID
            id: "cabcdd14-03ab-4ac0-b58c-dd4175bc587e".to_string(),
        };

        let Ok(data_str) = serde_json::to_string(&data) else {
            panic!("Failed to serialize data");
        };

        let uri = format!("{}/cargo/cancel", url);
        let Ok(req) = Request::builder()
            .method(Method::DELETE)
            .uri(uri.clone())
            .header("content-type", "application/json")
            .body(Body::from(data_str))
        else {
            panic!("Failed to build request");
        };

        let resp = client.request(req).await;
        let (success, result_str) = evaluate(resp, StatusCode::OK);
        ok &= success;

        println!("{}: {}", uri, result_str);
    }

    // PUT /cargo/scan
    {
        let data = ParcelScan {
            scanner_id: Uuid::new_v4().to_string(),
            parcel_id: Uuid::new_v4().to_string(),
            latitude: 52.37474373455002,
            longitude: 4.9167298573581295,
        };

        let Ok(data) = serde_json::to_string(&data) else {
            panic!("Failed to serialize data");
        };

        let uri = format!("{}/cargo/scan", url);
        let Ok(request) = Request::builder()
            .method(Method::PUT)
            .uri(uri.clone())
            .header("content-type", "application/json")
            .body(Body::from(data))
        else {
            panic!("Failed to build request");
        };

        let resp = client.request(request).await;
        let (success, result_str) = evaluate(resp, StatusCode::ACCEPTED);
        ok &= success;

        println!("{}: {}", uri, result_str);
    }

    // GET /cargo/flights
    {
        let data = LandingsQuery {
            vertiport_id: Uuid::new_v4().to_string(),
            arrival_window: Some(TimeWindow {
                timestamp_min: Utc::now(),
                timestamp_max: Utc::now() + Duration::seconds(3600),
            }),
            limit: 20,
        };

        let Ok(data) = serde_json::to_string(&data) else {
            panic!("Failed to serialize data");
        };

        let uri = format!("{}/cargo/landings", url);
        let Ok(request) = Request::builder()
            .method(Method::GET)
            .uri(uri.clone())
            .header("content-type", "application/json")
            .body(Body::from(data))
        else {
            panic!("Failed to build request");
        };

        let resp = client.request(request).await;
        let (success, result_str) = evaluate(resp, StatusCode::ACCEPTED);
        ok &= success;

        println!("{}: {}", uri, result_str);
    }

    if ok {
        println!("\u{1F9c1} All endpoints responded!");
    } else {
        eprintln!("\u{2620} Errors");
    }

    Ok(())
}
