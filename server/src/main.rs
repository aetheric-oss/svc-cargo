//! svc-cargo
//! Processes flight requests from client applications

use axum::{handler::Handler, routing, Router};
use hyper::Error;
use std::net::{Ipv4Addr, SocketAddr};
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;

mod rest;

/// GRPC Interfaces for svc-cargo
pub mod cargo_grpc {
    #![allow(unused_qualifications)]
    include!("grpc.rs");
}

use cargo_grpc::cargo_rpc_server::{CargoRpc, CargoRpcServer};

/// Struct that implements the CargoRpc trait.
///
/// This is the main struct that implements the gRPC service.
#[derive(Default, Debug, Clone, Copy)]
pub struct CargoGrpcImpl {}

// Implementing gRPC interfaces for this microservice
#[tonic::async_trait]
impl CargoRpc for CargoGrpcImpl {
    /// Replies true if this server is ready to serve others.
    /// # Arguments
    /// * `request` - the query object with no arguments
    /// # Returns
    /// * `ReadyResponse` - Returns true
    async fn is_ready(
        &self,
        _request: tonic::Request<cargo_grpc::QueryIsReady>,
    ) -> Result<tonic::Response<cargo_grpc::ReadyResponse>, tonic::Status> {
        let response = cargo_grpc::ReadyResponse { ready: true };
        Ok(tonic::Response::new(response))
    }
}

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

fn rest_server_start() {
    tokio::spawn(async move {
        let rest_port = std::env::var("DOCKER_PORT_REST")
            .unwrap_or_else(|_| "8000".to_string())
            .parse::<u16>()
            .unwrap_or(8000);
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

        let cors = CorsLayer::new()
            // allow `GET` and `POST` when accessing the resource
            .allow_methods(Any)
            .allow_headers(Any)
            // allow requests from any origin
            .allow_origin(Any);

        let app = Router::new()
            // .merge(SwaggerUi::new("/swagger-ui/*tail").url("/api-doc/openapi.json", ApiDoc::openapi()))
            .fallback(not_found.into_service())
            .route(rest::ENDPOINT_CANCEL, routing::delete(rest::cancel_flight))
            .route(rest::ENDPOINT_QUERY, routing::post(rest::query_flight))
            .route(rest::ENDPOINT_CONFIRM, routing::put(rest::confirm_flight))
            .route(
                rest::ENDPOINT_VERTIPORTS,
                routing::post(rest::query_vertiports),
            )
            .layer(cors);
        // header

        println!("REST API Hosted at 0.0.0.0:{rest_port}");
        let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, rest_port));
        axum::Server::bind(&address)
            .serve(app.into_make_service())
            .with_graceful_shutdown(shutdown_signal())
            .await
            .unwrap();
    });
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // REST API in background thread
    rest_server_start();

    // GRPC Server
    let grpc_port = std::env::var("DOCKER_PORT_GRPC")
        .unwrap_or_else(|_| "50051".to_string())
        .parse::<u16>()
        .unwrap_or(50051);

    let addr = format!("[::1]:{grpc_port}").parse().unwrap();
    let imp = CargoGrpcImpl::default();
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<CargoRpcServer<CargoGrpcImpl>>()
        .await;

    println!("gRPC Server Listening at {}", addr);
    tonic::transport::Server::builder()
        .add_service(health_service)
        .add_service(CargoRpcServer::new(imp))
        .serve_with_shutdown(addr, shutdown_signal())
        .await
        .unwrap();

    Ok(())
}
