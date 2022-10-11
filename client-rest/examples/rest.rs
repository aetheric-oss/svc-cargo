//! Example communication with this service

use chrono::{Duration, NaiveDate};
use hyper::StatusCode;
use hyper::{Body, Client, Method, Request};
use svc_cargo_client_rest::types::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("NOTE: Ensure the server is running, or this example will fail.");

    let rest_port = std::env::var("HOST_PORT").unwrap_or_else(|_| "8000".to_string());

    // let host_port = env!("HOST_PORT");
    let url = format!("http://0.0.0.0:{rest_port}");
    let mut ok = true;
    let client = Client::builder()
        .pool_idle_timeout(std::time::Duration::from_secs(10))
        .build_http();

    // GET /cargo/vertiports
    {
        let data = VertiportsQuery::new(32.7262, 117.1544);
        let data_str = serde_json::to_string(&data).unwrap();
        let uri = format!("{}{}", url, ENDPOINT_VERTIPORTS);
        let req = Request::builder()
            .method(Method::GET)
            .uri(uri.clone())
            .header("content-type", "application/json")
            .body(Body::from(data_str))
            .unwrap();

        let resp = client.request(req).await.unwrap();
        let tmp = resp.status() == StatusCode::OK;
        println!("{}: {}", uri, resp.status());
        assert!(tmp);
        ok &= tmp;
    }

    // PUT /cargo/confirm
    {
        let data = FlightConfirm {
            fp_id: "TEST".to_string(),
        };
        let data_str = serde_json::to_string(&data).unwrap();
        let uri = format!("{}{}", url, ENDPOINT_CONFIRM);
        let req = Request::builder()
            .method(Method::PUT)
            .uri(uri.clone())
            .header("content-type", "application/json")
            .body(Body::from(data_str))
            .unwrap();

        let resp = client.request(req).await.unwrap();
        let tmp = resp.status() == StatusCode::CREATED;
        println!("{}: {}", uri, resp.status());
        assert!(tmp);
        ok &= tmp;
    }

    // DELETE /cargo/cancel
    {
        let data = FlightCancel {
            fp_id: "TEST".to_string(),
        };
        let data_str = serde_json::to_string(&data).unwrap();
        let uri = format!("{}{}", url, ENDPOINT_CANCEL);
        let req = Request::builder()
            .method(Method::DELETE)
            .uri(uri.clone())
            .header("content-type", "application/json")
            .body(Body::from(data_str))
            .unwrap();

        let resp = client.request(req).await.unwrap();
        let tmp = resp.status() == StatusCode::OK;
        println!("{}: {}", uri, resp.status());
        assert!(tmp);
        ok &= tmp;
    }

    // GET /cargo/query
    {
        let depart_timestamp_min = NaiveDate::from_ymd(1999, 12, 31).and_hms(23, 59, 59);
        let data = FlightQuery::new(
            "vertiport_1".to_string(),
            "vertiport_2".to_string(),
            depart_timestamp_min,
            depart_timestamp_min + Duration::hours(1),
            1.0,
        );
        let data_str = serde_json::to_string(&data).unwrap();
        let uri = format!("{}{}", url, ENDPOINT_QUERY);
        let req = Request::builder()
            .method(Method::GET)
            .uri(uri.clone())
            .header("content-type", "application/json")
            .body(Body::from(data_str))
            .unwrap();

        let resp = client.request(req).await.unwrap();
        let tmp = resp.status() == StatusCode::OK;
        println!("{}: {}", uri, resp.status());
        ok &= tmp;
    }

    if ok {
        println!("\u{1F9c1} All endpoints responded!");
    } else {
        eprintln!("\u{2620} Errors");
    }

    Ok(())
}
