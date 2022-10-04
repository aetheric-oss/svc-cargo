//! svc-cargo
//! Processes flight requests from client applications

use axum::{handler::Handler, routing, Router, Server};
use hyper::Error;
use std::net::{Ipv4Addr, SocketAddr};
use utoipa::OpenApi;
// use utoipa_swagger_ui::SwaggerUi;

mod rest;

/// Tokio signal handler that will wait for a user to press CTRL+C.
/// We use this in our hyper `Server` method `with_graceful_shutdown`.
///
/// # Arguments
///
/// # Examples
///
/// ```
/// Server::bind(&"0.0.0.0:8000".parse().unwrap())
/// .serve(app.into_make_service())
/// .with_graceful_shutdown(shutdown_signal())
/// .await
/// .unwrap();
/// ```
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("expect tokio signal ctrl-c");
    println!("signal shutdown!");
}

/// Responds a NOT_FOUND status and error string
///
/// # Arguments
///
/// # Examples
///
/// ```
/// let app = Router::new()
///         .fallback(not_found.into_service());
/// ```
pub async fn not_found(uri: axum::http::Uri) -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::NOT_FOUND,
        format!("No route {}", uri),
    )
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    #[derive(OpenApi)]
    #[openapi(
        paths(
            rest::query_flight,
            rest::query_vertiports,
            rest::confirm_flight,
            rest::cancel_flight
        ),
        components(
            schemas(
                rest::FlightOption,
                rest::Vertiport,
                rest::ConfirmError,
                rest::VertiportsQuery,
                rest::FlightQuery
            )
        ),
        tags(
            (name = "svc-cargo", description = "svc-cargo API")
        )
    )]
    struct ApiDoc;

    let app = Router::new()
        // .merge(SwaggerUi::new("/swagger-ui/*tail").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .fallback(not_found.into_service())
        .route(rest::ENDPOINT_CANCEL, routing::delete(rest::cancel_flight))
        .route(rest::ENDPOINT_QUERY, routing::get(rest::query_flight))
        .route(rest::ENDPOINT_CONFIRM, routing::put(rest::confirm_flight))
        .route(
            rest::ENDPOINT_VERTIPORTS,
            routing::get(rest::query_vertiports),
        );

    let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8000));
    Server::bind(&address)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
}
