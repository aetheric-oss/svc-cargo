//! gRPC client implementation

use svc_cargo_client::client::{cargo_rpc_client::CargoRpcClient, QueryIsReady};

/// Example svc-cargo-client
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("NOTE: Ensure the server is running, or this example will fail.");
    let mut ok = true;

    let grpc_port = std::env::var("HOST_PORT_GRPC").unwrap_or_else(|_| "50051".to_string());
    let mut client = CargoRpcClient::connect(format!("http://[::1]:{grpc_port}")).await?;
    let request = tonic::Request::new(QueryIsReady {});
    let response = client.is_ready(request).await;
    if response.is_err() {
        ok = false;
        println!("IsReady: FAIL");
    } else {
        println!("IsReady: PASS");
    }

    if ok {
        println!("\u{1F9c1} All endpoints responded!");
    } else {
        eprintln!("\u{2620} Errors");
    }

    Ok(())
}
