//! # Config
//!
//! Define and implement config options for module

use anyhow::Result;
use config::{ConfigError, Environment};
use dotenv::dotenv;
use serde::Deserialize;

/// struct holding configuration options
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// port to be used for gRPC server
    pub docker_port_grpc: u16,
    /// port to be used for REST server
    pub docker_port_rest: u16,
    /// port to be used for the storage client
    pub storage_port_grpc: u16,
    /// host to be used for the storage client
    pub storage_host_grpc: String,
    /// port to be used for the pricing client
    pub pricing_port_grpc: u16,
    /// host to be used for the pricing client
    pub pricing_host_grpc: String,
    /// port to be used for the scheduler client
    pub scheduler_port_grpc: u16,
    /// host to be used for the scheduler client
    pub scheduler_host_grpc: String,
    /// path to log configuration YAML file
    pub log_config: String,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    /// Default values for Config
    pub fn new() -> Self {
        Config {
            docker_port_grpc: 50051,
            docker_port_rest: 8000,
            storage_port_grpc: 50003,
            storage_host_grpc: String::from("svc-storage"),
            pricing_port_grpc: 50001,
            pricing_host_grpc: String::from("svc-pricing"),
            scheduler_port_grpc: 50002,
            scheduler_host_grpc: String::from("svc-scheduler"),
            log_config: String::from("log4rs.yaml"),
        }
    }

    /// Create a new `Config` object using environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        // read .env file if present
        dotenv().ok();

        config::Config::builder()
            .set_default("docker_port_grpc", 50051)?
            .set_default("docker_port_rest", 8000)?
            .set_default("storage_port_grpc", 50003)?
            .set_default("storage_host_grpc", String::from("svc-storage"))?
            .set_default("pricing_port_grpc", 50001)?
            .set_default("pricing_host_grpc", String::from("svc-pricing"))?
            .set_default("scheduler_port_grpc", 50002)?
            .set_default("scheduler_host_grpc", String::from("svc-scheduler"))?
            .set_default("log_config", String::from("log4rs.yaml"))?
            .add_source(Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }
}
