pub use svc_pricing_client::pricing_grpc::pricing_client::PricingClient;
pub use svc_scheduler_client_grpc::grpc::rpc_service_client::RpcServiceClient as SchedulerClient;
pub use svc_storage_client_grpc::{ParcelClient, ParcelScanClient, VertiportClient};

use futures::lock::Mutex;
use std::sync::Arc;
pub use tonic::transport::Channel;

#[derive(Clone, Debug)]
pub struct GrpcClients {
    pub scheduler: GrpcClient<SchedulerClient<Channel>>,
    pub vertiport_storage: GrpcClient<VertiportClient<Channel>>,
    pub parcel_storage: GrpcClient<ParcelClient<Channel>>,
    pub parcel_scan_storage: GrpcClient<ParcelScanClient<Channel>>,
    pub pricing: GrpcClient<PricingClient<Channel>>,
}

#[derive(Debug, Clone)]
pub struct GrpcClient<T> {
    inner: Arc<Mutex<Option<T>>>,
    address: String,
}

impl<T> GrpcClient<T> {
    pub async fn invalidate(&mut self) {
        let arc = Arc::clone(&self.inner);
        let mut client = arc.lock().await;
        *client = None;
    }

    pub fn new(host: &str, port: u16) -> Self {
        let opt: Option<T> = None;
        GrpcClient {
            inner: Arc::new(Mutex::new(opt)),
            address: format!("http://{host}:{port}"),
        }
    }
}

macro_rules! grpc_client {
    ( $client: ident, $name: expr ) => {
        impl GrpcClient<$client<Channel>> {
            pub async fn get_client(&mut self) -> Option<$client<Channel>> {
                grpc_info!("(get_client) storage::{} entry.", $name);

                let arc = Arc::clone(&self.inner);

                // if already connected, return the client
                let client = arc.lock().await;
                if client.is_some() {
                    return client.clone();
                }

                grpc_info!(
                    "(grpc) connecting to {} server at {}.",
                    $name,
                    self.address.clone()
                );
                let result = $client::connect(self.address.clone()).await;
                match result {
                    Ok(client) => {
                        grpc_info!(
                            "(grpc) success: connected to {} server at {}.",
                            $name,
                            self.address.clone()
                        );
                        Some(client)
                    }
                    Err(e) => {
                        grpc_error!(
                            "(grpc) couldn't connect to {} server at {}; {}.",
                            $name,
                            self.address,
                            e
                        );
                        None
                    }
                }
            }
        }
    };
}

grpc_client!(SchedulerClient, "scheduler");
grpc_client!(VertiportClient, "vertiport_storage");
grpc_client!(ParcelScanClient, "parcel_scan_storage");
grpc_client!(ParcelClient, "parcel_storage");
grpc_client!(PricingClient, "pricing");

impl GrpcClients {
    pub fn new(config: crate::config::Config) -> Self {
        GrpcClients {
            scheduler: GrpcClient::<SchedulerClient<Channel>>::new(
                &config.scheduler_host_grpc,
                config.scheduler_port_grpc,
            ),
            // vertiport storage
            vertiport_storage: GrpcClient::<VertiportClient<Channel>>::new(
                &config.storage_host_grpc,
                config.storage_port_grpc,
            ),
            // parcel storage
            parcel_storage: GrpcClient::<ParcelClient<Channel>>::new(
                &config.storage_host_grpc,
                config.storage_port_grpc,
            ),
            // vertiport storage
            parcel_scan_storage: GrpcClient::<ParcelScanClient<Channel>>::new(
                &config.storage_host_grpc,
                config.storage_port_grpc,
            ),
            pricing: GrpcClient::<PricingClient<Channel>>::new(
                &config.pricing_host_grpc,
                config.pricing_port_grpc,
            ),
        }
    }
}
