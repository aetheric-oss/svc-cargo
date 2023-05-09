use crate::grpc::client::GrpcClients;
use axum::extract::Extension;
use hyper::StatusCode;
use svc_storage_client_grpc::ClientConnect;

#[utoipa::path(
    get,
    path = "/health",
    tag = "svc-cargo",
    responses(
        (status = 200, description = "Service is healthy, all dependencies running."),
        (status = 503, description = "Service is unhealthy, one or more dependencies unavailable.")
    )
)]
pub async fn health_check(
    Extension(mut grpc_clients): Extension<GrpcClients>,
) -> Result<(), StatusCode> {
    rest_debug!("(health_check) entry.");

    let mut ok = true;

    if grpc_clients.storage.vertiport.get_client().await.is_err() {
        let error_msg = "svc-storage vertiport client unavailable.".to_string();
        rest_error!("(health_check) {}", &error_msg);
        ok = false;
    };

    if grpc_clients.storage.vertipad.get_client().await.is_err() {
        let error_msg = "svc-storage vertipad client unavailable.".to_string();
        rest_error!("(health_check) {}", &error_msg);
        ok = false;
    };

    if grpc_clients.storage.parcel.get_client().await.is_err() {
        let error_msg = "svc-storage parcel client unavailable.".to_string();
        rest_error!("(health_check) {}", &error_msg);
        ok = false;
    };

    if grpc_clients.storage.parcel_scan.get_client().await.is_err() {
        let error_msg = "svc-storage parcel scan client unavailable.".to_string();
        rest_error!("(health_check) {}", &error_msg);
        ok = false;
    };

    if grpc_clients.storage.flight_plan.get_client().await.is_err() {
        let error_msg = "svc-storage flight_plan client unavailable.".to_string();
        rest_error!("(health_check) {}", &error_msg);
        ok = false;
    };

    if grpc_clients.storage.vehicle.get_client().await.is_err() {
        let error_msg = "svc-storage vehicle client unavailable.".to_string();
        rest_error!("(health_check) {}", &error_msg);
        ok = false;
    };

    let result = grpc_clients.pricing.get_client().await;
    if result.is_none() {
        let error_msg = "svc-pricing unavailable.".to_string();
        rest_error!("(health_check) {}", &error_msg);
        ok = false;
    };

    let result = grpc_clients.scheduler.get_client().await;
    if result.is_none() {
        let error_msg = "svc-scheduler unavailable.".to_string();
        rest_error!("(health_check) {}", &error_msg);
        ok = false;
    };

    match ok {
        true => {
            rest_info!("(health_check) healthy, all dependencies running.");
            Ok(())
        }
        false => {
            rest_error!("(health_check) unhealthy, 1+ dependencies down.");
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}
