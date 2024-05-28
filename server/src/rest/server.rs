//! Rest server implementation

use super::api;
use crate::grpc::client::get_clients;
use crate::shutdown_signal;
use crate::Config;
use axum::{
    error_handling::HandleErrorLayer,
    extract::Extension,
    http::{HeaderValue, StatusCode},
    routing, BoxError, Router,
};
use std::net::SocketAddr;
use tower::{
    buffer::BufferLayer,
    limit::{ConcurrencyLimitLayer, RateLimitLayer},
    ServiceBuilder,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Starts the REST API server for this microservice
///
/// # Example:
/// ```
/// use svc_cargo::rest::server::rest_server;
/// use svc_cargo::Config;
/// async fn example() -> Result<(), tokio::task::JoinError> {
///     let config = Config::default();
///     tokio::spawn(rest_server(config, None)).await;
///     Ok(())
/// }
/// ```
pub async fn rest_server(
    config: Config,
    shutdown_rx: Option<tokio::sync::oneshot::Receiver<()>>,
) -> Result<(), ()> {
    rest_info!("entry.");
    let rest_port = config.docker_port_rest;
    let full_rest_addr: SocketAddr = format!("[::]:{}", rest_port).parse().map_err(|e| {
        rest_error!("invalid address: {:?}, exiting.", e);
    })?;

    let cors_allowed_origin = config
        .rest_cors_allowed_origin
        .parse::<HeaderValue>()
        .map_err(|e| {
            rest_error!("invalid cors_allowed_origin address: {:?}, exiting.", e);
        })?;

    // Rate limiting
    let rate_limit = config.rest_request_limit_per_second as u64;
    let concurrency_limit = config.rest_concurrency_limit_per_service as usize;
    let limit_middleware = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(HandleErrorLayer::new(|e: BoxError| async move {
            rest_warn!("too many requests: {}", e);
            (
                StatusCode::TOO_MANY_REQUESTS,
                "(server) too many requests.".to_string(),
            )
        }))
        .layer(BufferLayer::new(100))
        .layer(ConcurrencyLimitLayer::new(concurrency_limit))
        .layer(RateLimitLayer::new(
            rate_limit,
            std::time::Duration::from_secs(1),
        ));

    //
    // Extensions
    //
    // GRPC Clients
    let grpc_clients = get_clients().await;

    let app = Router::new()
        .route("/health", routing::get(api::health::health_check))
        .route(
            "/cargo/cancel",
            routing::delete(api::cancel::cancel_itinerary),
        )
        .route(
            "/cargo/request",
            routing::post(api::request::request_flight),
        )
        .route("/cargo/create", routing::put(api::create::create_itinerary))
        .route(
            "/cargo/vertiports",
            routing::post(api::query::query_vertiports),
        )
        .route("/cargo/scan", routing::put(api::scan::scan_parcel))
        .route("/cargo/track/:id", routing::get(api::query::query_scans))
        .route(
            "/cargo/occupations",
            routing::post(api::query::query_occupations),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(cors_allowed_origin)
                .allow_headers(Any)
                .allow_methods(Any),
        )
        .layer(limit_middleware)
        .layer(Extension(grpc_clients)); // Extension layer must be last

    //
    // Bind to address
    //
    match axum::Server::bind(&full_rest_addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal("rest", shutdown_rx))
        .await
    {
        Ok(_) => {
            rest_info!("hosted at: {}.", full_rest_addr);
            Ok(())
        }
        Err(e) => {
            rest_error!("could not start server: {}", e);
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_start_and_shutdown() {
        use tokio::time::{sleep, Duration};
        lib_common::logger::get_log_handle().await;
        ut_info!("start");

        let config = Config::default();

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Start the rest server
        tokio::spawn(rest_server(config, Some(shutdown_rx)));

        // Give the server time to get through the startup sequence (and thus code)
        sleep(Duration::from_secs(1)).await;

        // Shut down server
        assert!(shutdown_tx.send(()).is_ok());

        ut_info!("success");
    }
}
