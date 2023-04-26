use super::api;
use crate::grpc::client::GrpcClients;
use svc_cargo::shutdown_signal;

use axum::{extract::Extension, routing, Router};

/// Starts the REST API server for this microservice
#[cfg(not(tarpaulin_include))]
pub async fn server(config: crate::config::Config) {
    rest_info!("(rest) starting server.");

    let rest_port = config.docker_port_rest;

    // Wait for other GRPC Servers
    let grpc_clients = GrpcClients::new(config);

    let app = Router::new()
        .route("/health", routing::get(api::health_check))
        .route("/cargo/cancel", routing::delete(api::cancel_itinerary))
        .route("/cargo/query", routing::post(api::query_flight))
        .route("/cargo/confirm", routing::put(api::confirm_itinerary))
        .route("/cargo/vertiports", routing::post(api::query_vertiports))
        .layer(Extension(grpc_clients)); // Extension layer must be last

    let address = format!("[::]:{rest_port}");
    let Ok(address) = address.parse() else {
        rest_error!("(rest server) failed to parse address: {}", address);
        return;
    };

    rest_info!("(rest server) hosted at {:?}", address);
    let _ = axum::Server::bind(&address)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal("rest"))
        .await;
}
