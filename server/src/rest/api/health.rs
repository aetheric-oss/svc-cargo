use crate::grpc::client::GrpcClients;
use axum::extract::Extension;
use hyper::StatusCode;

use svc_scheduler_client_grpc::prelude::{scheduler, SchedulerServiceClient};
use svc_storage_client_grpc::prelude::{ReadyRequest, SimpleClient};

#[utoipa::path(
    get,
    path = "/health",
    tag = "svc-cargo",
    responses(
        (status = 200, description = "Service is healthy, all dependencies running."),
        (status = 503, description = "Service is unhealthy, one or more dependencies unavailable.")
    )
)]
#[cfg(not(tarpaulin_include))]
// no_coverage: (R5) need backends to test failures (integration)
pub async fn health_check(
    Extension(grpc_clients): Extension<GrpcClients>,
) -> Result<(), StatusCode> {
    rest_debug!("entry.");

    let mut ok = true;

    // This health check is to verify that ALL dependencies of this
    // microservice are running.
    if grpc_clients
        .storage
        .vertiport
        .is_ready(ReadyRequest {})
        .await
        .is_err()
    {
        let error_msg = "svc-storage vertiport unavailable.".to_string();
        rest_error!("{}.", &error_msg);
        ok = false;
    }

    if grpc_clients
        .storage
        .vertipad
        .is_ready(ReadyRequest {})
        .await
        .is_err()
    {
        let error_msg = "svc-storage vertipad unavailable.".to_string();
        rest_error!("{}.", &error_msg);
        ok = false;
    };

    if grpc_clients
        .storage
        .parcel
        .is_ready(ReadyRequest {})
        .await
        .is_err()
    {
        let error_msg = "svc-storage parcel unavailable.".to_string();
        rest_error!("{}.", &error_msg);
        ok = false;
    };

    if grpc_clients
        .storage
        .parcel_scan
        .is_ready(ReadyRequest {})
        .await
        .is_err()
    {
        let error_msg = "svc-storage parcel_scan unavailable.".to_string();
        rest_error!("{}.", &error_msg);
        ok = false;
    };

    if grpc_clients
        .storage
        .flight_plan
        .is_ready(ReadyRequest {})
        .await
        .is_err()
    {
        let error_msg = "svc-storage flight_plan unavailable.".to_string();
        rest_error!("{}.", &error_msg);
        ok = false;
    }

    if grpc_clients
        .storage
        .vehicle
        .is_ready(ReadyRequest {})
        .await
        .is_err()
    {
        let error_msg = "svc-storage vehicle unavailable.".to_string();
        rest_error!("{}.", &error_msg);
        ok = false;
    };

    if grpc_clients
        .scheduler
        .is_ready(scheduler::ReadyRequest {})
        .await
        .is_err()
    {
        let error_msg = "svc-scheduler client unavailable.".to_string();
        rest_error!("{}", &error_msg);
        ok = false;
    };

    match ok {
        true => {
            rest_debug!("healthy, all dependencies running.");
            Ok(())
        }
        false => {
            rest_error!("unhealthy, 1+ dependencies down.");
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_success() {
        lib_common::logger::get_log_handle().await;
        ut_info!("start");

        // Mock the GrpcClients extension
        let config = crate::Config::default();
        let grpc_clients = GrpcClients::default(config); // Replace with your own mock implementation

        // Call the health_check function
        let result = health_check(Extension(grpc_clients)).await;

        // Assert the expected result
        println!("{:?}", result);
        assert!(result.is_ok());

        ut_info!("success");
    }
}
