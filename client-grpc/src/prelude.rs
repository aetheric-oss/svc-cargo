//! Re-export of used objects

pub use super::client as cargo;
pub use super::service::Client as CargoServiceClient;
pub use cargo::CargoClient;

pub use lib_common::grpc::Client;
