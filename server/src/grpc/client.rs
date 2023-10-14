use lib_common::grpc::{Client, GrpcClient};
pub use svc_pricing_client_grpc::client::rpc_service_client::RpcServiceClient as PricingClient;
pub use svc_scheduler_client_grpc::client::rpc_service_client::RpcServiceClient as SchedulerClient;
use svc_storage_client_grpc::Clients;
pub use tonic::transport::Channel;

#[derive(Clone, Debug)]
pub struct GrpcClients {
    /// All clients enabled from the svc_storage_grpc_client module
    pub storage: Clients,
    /// A GrpcClient provided by the svc_scheduler_grpc_client module
    pub scheduler: GrpcClient<SchedulerClient<Channel>>,
    /// A GrpcClient provided by the svc_pricing_grpc_client module
    pub pricing: GrpcClient<PricingClient<Channel>>,
}

impl GrpcClients {
    pub fn default(config: crate::config::Config) -> Self {
        GrpcClients {
            storage: Clients::new(config.storage_host_grpc, config.storage_port_grpc),
            scheduler: GrpcClient::<SchedulerClient<Channel>>::new_client(
                &config.scheduler_host_grpc,
                config.scheduler_port_grpc,
                "scheduler",
            ),
            pricing: GrpcClient::<PricingClient<Channel>>::new_client(
                &config.pricing_host_grpc,
                config.pricing_port_grpc,
                "pricing",
            ),
        }
    }
}
