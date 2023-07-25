//! gRPC client helpers implementation
use lib_common::grpc::{Client, GrpcClient};
use svc_pricing_client_grpc::client::rpc_service_client::RpcServiceClient as PricingClient;
use svc_scheduler_client_grpc::client::rpc_service_client::RpcServiceClient as SchedulerClient;
use svc_storage_client_grpc::Clients;
use tokio::sync::OnceCell;
use tonic::transport::Channel;

pub(crate) static CLIENTS: OnceCell<GrpcClients> = OnceCell::const_new();

/// Returns CLIENTS, a GrpcClients object with default values.
/// Uses host and port configurations using a Config object generated from
/// environment variables.
/// Initializes CLIENTS if it hasn't been initialized yet.
pub async fn get_clients() -> &'static GrpcClients {
    CLIENTS
        .get_or_init(|| async move {
            let config = crate::Config::try_from_env().unwrap_or_default();
            GrpcClients::default(config)
        })
        .await
}

/// Struct to hold all gRPC client connections
#[derive(Clone, Debug)]
pub struct GrpcClients {
    /// All clients enabled from the svc_storage_grpc_client module
    pub storage: Clients,
    /// A GrpcClient provided by the svc_pricing module
    pub pricing: GrpcClient<PricingClient<Channel>>,
    /// A GrpcClient provided by the svc_scheduler module
    pub scheduler: GrpcClient<SchedulerClient<Channel>>,
}

impl GrpcClients {
    /// Create new GrpcClients with defaults
    pub fn default(config: crate::Config) -> Self {
        let storage_clients = Clients::new(config.storage_host_grpc, config.storage_port_grpc);

        GrpcClients {
            storage: storage_clients,
            pricing: GrpcClient::<PricingClient<Channel>>::new_client(
                &config.pricing_host_grpc,
                config.pricing_port_grpc,
                "pricing",
            ),
            scheduler: GrpcClient::<SchedulerClient<Channel>>::new_client(
                &config.scheduler_host_grpc,
                config.scheduler_port_grpc,
                "scheduler",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{init_logger, Config};

    use svc_storage_client_grpc::Client;

    #[tokio::test]
    async fn test_grpc_clients_default() {
        init_logger(&Config::try_from_env().unwrap_or_default());
        unit_test_info!("(test_grpc_clients_default) start");

        let clients = get_clients().await;

        let vehicle = &clients.storage.vehicle;
        println!("{:?}", vehicle);
        assert_eq!(vehicle.get_name(), "vehicle");

        let vertipad = &clients.storage.vertipad;
        println!("{:?}", vertipad);
        assert_eq!(vertipad.get_name(), "vertipad");

        let vertiport = &clients.storage.vertiport;
        println!("{:?}", vertiport);
        assert_eq!(vertiport.get_name(), "vertiport");

        let parcel_scan = &clients.storage.parcel_scan;
        println!("{:?}", parcel_scan);
        assert_eq!(parcel_scan.get_name(), "parcel_scan");

        let parcel = &clients.storage.parcel;
        println!("{:?}", parcel);
        assert_eq!(parcel.get_name(), "parcel");

        let flight_plan = &clients.storage.flight_plan;
        println!("{:?}", flight_plan);
        assert_eq!(flight_plan.get_name(), "flight_plan");

        let scheduler = &clients.scheduler;
        println!("{:?}", scheduler);
        assert_eq!(scheduler.get_name(), "scheduler");

        let pricing = &clients.pricing;
        println!("{:?}", pricing);
        assert_eq!(pricing.get_name(), "pricing");

        unit_test_info!("(test_grpc_clients_default) success");
    }
}
