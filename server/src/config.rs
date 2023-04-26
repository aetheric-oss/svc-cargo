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

impl Config {
    /// Create a new `Config` object using environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        // read .env file if present
        dotenv().ok();

        config::Config::builder()
            .add_source(Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }
}
