mod grpc_server {
    #![allow(unused_qualifications)]
    tonic::include_proto!("grpc");
}

use grpc_server::rpc_service_server::{RpcService, RpcServiceServer};

use svc_cargo::shutdown_signal;

/// Struct that implements the CargoRpc trait.
///
/// This is the main struct that implements the gRPC service.
#[derive(Default, Debug, Clone, Copy)]
pub struct ServiceImpl {}

// Implementing gRPC interfaces for this microservice
#[tonic::async_trait]
impl RpcService for ServiceImpl {
    /// Replies true if this server is ready to serve others.
    async fn is_ready(
        &self,
        _request: tonic::Request<grpc_server::ReadyRequest>,
    ) -> Result<tonic::Response<grpc_server::ReadyResponse>, tonic::Status> {
        grpc_info!("is_ready() enter");
        let response = grpc_server::ReadyResponse { ready: true };

        grpc_info!("is_ready() exit");
        Ok(tonic::Response::new(response))
    }
}

/// Starts the grpc server for this microservice
#[cfg(not(tarpaulin_include))]
pub async fn server(config: crate::config::Config) {
    // GRPC Server
    let grpc_port = config.docker_port_grpc;
    let addr = format!("[::]:{grpc_port}");
    let Ok(addr) = addr.parse() else {
        grpc_error!("(grpc server) failed to parse address: {}", addr);
        return;
    };

    let imp = ServiceImpl::default();
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<RpcServiceServer<ServiceImpl>>()
        .await;

    grpc_info!("(grpc server) hosted at {}", addr);
    let _ = tonic::transport::Server::builder()
        .add_service(health_service)
        .add_service(RpcServiceServer::new(imp))
        .serve_with_shutdown(addr, shutdown_signal("grpc"))
        .await;
}
