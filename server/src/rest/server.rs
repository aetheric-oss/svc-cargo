use super::api;
use crate::grpc::client::GrpcClients;
use axum::{
    error_handling::HandleErrorLayer, extract::Extension, http::StatusCode, routing, BoxError,
    Router,
};
use svc_cargo::shutdown_signal;
use tower_http::cors::{Any, CorsLayer};

use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};

/// Starts the REST API server for this microservice
#[cfg(not(tarpaulin_include))]
pub async fn server(config: crate::config::Config) {
    rest_info!("(server) starting server.");

    let rest_port = config.docker_port_rest;
    let rate_limit = config.request_limit_per_second as u64;

    // Wait for other GRPC Servers
    let grpc_clients = GrpcClients::default(config);

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
        .route(
            "/cargo/confirm",
            routing::put(api::confirm::confirm_itinerary),
        )
        .route(
            "/cargo/vertiports",
            routing::post(api::query::query_vertiports),
        )
        .route("/cargo/scan", routing::put(api::scan::scan_parcel))
        .route("/cargo/track", routing::get(api::query::query_scans))
        .route("/cargo/landings", routing::get(api::query::query_landings))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods(Any),
        )
        .layer(
            // Rate limiting
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|e: BoxError| async move {
                    rest_warn!("(server) too many requests: {}", e);
                    (
                        StatusCode::TOO_MANY_REQUESTS,
                        "(server) too many requests.".to_string(),
                    )
                }))
                .layer(BufferLayer::new(100))
                .layer(RateLimitLayer::new(
                    rate_limit,
                    std::time::Duration::from_secs(1),
                )),
        )
        .layer(Extension(grpc_clients)); // Extension layer must be last;

    let address = format!("[::]:{rest_port}");
    let Ok(address) = address.parse() else {
        rest_error!("(server) failed to parse address: {}", address);
        return;
    };

    rest_info!("(server) hosted at {:?}", address);
    let _ = axum::Server::bind(&address)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal("rest"))
        .await;
}
