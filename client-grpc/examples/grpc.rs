//! gRPC client implementation

///module svc_scheduler generated from svc-scheduler.proto
// use std::time::SystemTime;
// use svc_cargo_client_grpc::client::cargo_rpc_client::CargoRpcClient;

/// Example svc-cargo-client
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let mut client = CargoRpcClient::connect("http://[::1]:50051").await?;
    // let sys_time = SystemTime::now();
    // let request = tonic::Request::new(QueryFlightRequest {
    //     is_cargo: true,
    //     persons: 0,
    //     weight_grams: 5000,
    //     latitude: 37.77397,
    //     longitude: -122.43129,
    //     requested_time: Some(prost_types::Timestamp::from(sys_time)),
    // });

    // let response = client.query_flight(request).await?;

    // println!("RESPONSE={:?}", response.into_inner());

    Ok(())
}
