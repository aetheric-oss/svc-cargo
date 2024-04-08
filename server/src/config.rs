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
    /// Rate limit - requests per second for REST requests
    pub rest_request_limit_per_second: u8,
    /// Enforces a limit on the concurrent number of requests the underlying service can handle
    pub rest_concurrency_limit_per_service: u8,
    /// Full url (including port number) to be allowed as request origin for
    /// REST requests
    pub rest_cors_allowed_origin: String,
    /// config to be used for the Redis server
    pub redis: deadpool_redis::Config,
}

impl Default for Config {
    fn default() -> Self {
        log::warn!("(default) Creating Config object with default values.");
        Self::new()
    }
}

impl Config {
    /// Default values for Config
    pub fn new() -> Self {
        Config {
            docker_port_grpc: 50051,
            docker_port_rest: 8000,
            storage_port_grpc: 50051,
            storage_host_grpc: String::from("svc-storage"),
            pricing_port_grpc: 50051,
            pricing_host_grpc: String::from("svc-pricing"),
            scheduler_port_grpc: 50051,
            scheduler_host_grpc: String::from("svc-scheduler"),
            log_config: String::from("log4rs.yaml"),
            rest_request_limit_per_second: 2,
            rest_concurrency_limit_per_service: 5,
            rest_cors_allowed_origin: String::from("http://localhost:3000"),
            redis: deadpool_redis::Config {
                url: None,
                pool: None,
                connection: None,
            },
        }
    }

    /// Create a new `Config` object using environment variables
    pub fn try_from_env() -> Result<Self, ConfigError> {
        // read .env file if present
        dotenv().ok();
        let default_config = Config::default();

        config::Config::builder()
            .set_default("docker_port_grpc", default_config.docker_port_grpc)?
            .set_default("docker_port_rest", default_config.docker_port_rest)?
            .set_default("storage_port_grpc", default_config.storage_port_grpc)?
            .set_default("storage_host_grpc", default_config.storage_host_grpc)?
            .set_default("pricing_port_grpc", default_config.pricing_port_grpc)?
            .set_default("pricing_host_grpc", default_config.pricing_host_grpc)?
            .set_default("scheduler_port_grpc", default_config.scheduler_port_grpc)?
            .set_default("scheduler_host_grpc", default_config.scheduler_host_grpc)?
            .set_default("log_config", default_config.log_config)?
            .set_default(
                "rest_concurrency_limit_per_service",
                default_config.rest_concurrency_limit_per_service,
            )?
            .set_default(
                "rest_request_limit_per_seconds",
                default_config.rest_request_limit_per_second,
            )?
            .set_default(
                "rest_cors_allowed_origin",
                default_config.rest_cors_allowed_origin,
            )?
            .add_source(Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[tokio::test]
    async fn test_config_from_default() {
        crate::get_log_handle().await;
        ut_info!("(test_config_from_default) Start.");

        let config = Config::default();

        assert_eq!(config.docker_port_grpc, 50051);
        assert_eq!(config.docker_port_rest, 8000);
        assert_eq!(config.storage_port_grpc, 50051);
        assert_eq!(config.storage_host_grpc, String::from("svc-storage"));
        assert_eq!(config.pricing_port_grpc, 50051);
        assert_eq!(config.pricing_host_grpc, String::from("svc-pricing"));
        assert_eq!(config.scheduler_port_grpc, 50051);
        assert_eq!(config.scheduler_host_grpc, String::from("svc-scheduler"));
        assert_eq!(config.log_config, String::from("log4rs.yaml"));
        assert_eq!(config.rest_concurrency_limit_per_service, 5);
        assert_eq!(config.rest_request_limit_per_second, 2);
        assert_eq!(
            config.rest_cors_allowed_origin,
            String::from("http://localhost:3000")
        );
        assert!(config.redis.url.is_none());
        assert!(config.redis.pool.is_none());
        assert!(config.redis.connection.is_none());

        ut_info!("(test_config_from_default) Success.");
    }

    #[tokio::test]
    async fn test_config_from_env() {
        crate::get_log_handle().await;
        ut_info!("(test_config_from_env) Start.");

        std::env::set_var("DOCKER_PORT_GRPC", "6789");
        std::env::set_var("DOCKER_PORT_REST", "9876");
        std::env::set_var("STORAGE_HOST_GRPC", "test_host_storage");
        std::env::set_var("STORAGE_PORT_GRPC", "12345");
        std::env::set_var("PRICING_HOST_GRPC", "test_host_pricing");
        std::env::set_var("PRICING_PORT_GRPC", "54321");
        std::env::set_var("SCHEDULER_HOST_GRPC", "test_host_scheduler");
        std::env::set_var("SCHEDULER_PORT_GRPC", "12354");
        std::env::set_var("LOG_CONFIG", "config_file.yaml");
        std::env::set_var("REST_CONCURRENCY_LIMIT_PER_SERVICE", "255");
        std::env::set_var("REST_REQUEST_LIMIT_PER_SECOND", "255");
        std::env::set_var(
            "REST_CORS_ALLOWED_ORIGIN",
            "https://allowed.origin.host:443",
        );
        std::env::set_var("REDIS__URL", "redis://test_redis:6379");
        std::env::set_var("REDIS__POOL__MAX_SIZE", "16");
        std::env::set_var("REDIS__POOL__TIMEOUTS__WAIT__SECS", "2");
        std::env::set_var("REDIS__POOL__TIMEOUTS__WAIT__NANOS", "0");

        let config = Config::try_from_env();
        assert!(config.is_ok());
        let config = config.unwrap();

        assert_eq!(config.docker_port_grpc, 6789);
        assert_eq!(config.docker_port_rest, 9876);
        assert_eq!(config.storage_port_grpc, 12345);
        assert_eq!(config.storage_host_grpc, String::from("test_host_storage"));
        assert_eq!(config.pricing_port_grpc, 54321);
        assert_eq!(config.pricing_host_grpc, String::from("test_host_pricing"));
        assert_eq!(config.scheduler_port_grpc, 12354);
        assert_eq!(
            config.scheduler_host_grpc,
            String::from("test_host_scheduler")
        );
        assert_eq!(config.log_config, String::from("config_file.yaml"));
        assert_eq!(config.rest_concurrency_limit_per_service, 255);
        assert_eq!(config.rest_request_limit_per_second, 255);
        assert_eq!(
            config.rest_cors_allowed_origin,
            String::from("https://allowed.origin.host:443")
        );
        assert_eq!(
            config.redis.url,
            Some(String::from("redis://test_redis:6379"))
        );
        assert!(config.redis.pool.is_some());

        ut_info!("(test_config_from_env) Success.");
    }
}
