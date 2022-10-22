use svc_pricing_client::pricing_grpc::pricing_client::PricingClient;
use svc_scheduler_client::svc_scheduler::scheduler_client::SchedulerClient;
use svc_storage_client_grpc::client::storage_rpc_client::StorageRpcClient;
use tonic::transport::Channel;

pub use svc_storage_client_grpc::client::VertiportFilter;

#[derive(Clone, Debug)]
pub struct GrpcClients {
    pub scheduler: SchedulerClient<Channel>,
    pub storage: StorageRpcClient<Channel>,
    pub pricing: PricingClient<Channel>,
}

fn get_grpc_endpoint(env_port: &str, default: u16) -> String {
    let port = std::env::var(env_port)
        .unwrap_or_else(|_| default.to_string())
        .parse::<u16>()
        .unwrap_or(default);
    let url: String = "[::]".to_string();
    format!("http://{url}:{port}")
}

impl GrpcClients {
    pub async fn create() -> Result<Self, tonic::transport::Error> {
        let scheduler_endpoint = get_grpc_endpoint("SCHEDULER_PORT_GRPC", 8001);
        let pricing_endpoint = get_grpc_endpoint("PRICING_PORT_GRPC", 8003);
        let storage_endpoint = get_grpc_endpoint("STORAGE_PORT_GRPC", 8000);

        let scheduler = SchedulerClient::connect(scheduler_endpoint).await?;
        let storage = StorageRpcClient::connect(storage_endpoint).await?;
        let pricing = PricingClient::connect(pricing_endpoint).await?;

        Ok(GrpcClients {
            scheduler,
            storage,
            pricing,
        })
    }
}
