//! <center>
//! <img src="https://github.com/Arrow-air/tf-github/raw/main/src/templates/doc-banner-services.png" style="height:250px" />
//! </center>
//! <div align="center">
//!     <a href="https://github.com/Arrow-air/svc-cargo/releases">
//!         <img src="https://img.shields.io/github/v/release/Arrow-air/svc-cargo?include_prereleases" alt="GitHub release (latest by date including pre-releases)">
//!     </a>
//!     <a href="https://github.com/Arrow-air/svc-cargo/tree/main">
//!         <img src="https://github.com/arrow-air/svc-cargo/actions/workflows/rust_ci.yml/badge.svg?branch=main" alt="Rust Checks">
//!     </a>
//!     <a href="https://discord.com/invite/arrow">
//!         <img src="https://img.shields.io/discord/853833144037277726?style=plastic" alt="Arrow DAO Discord">
//!     </a>
//!     <br><br>
//! </div>
//!
//! svc-cargo
//! Processes flight requests from client applications

mod config;
mod grpc;
mod rest;
mod rest_types {
    include!("../../openapi/types.rs");
}

use clap::Parser;
use dotenv::dotenv;
use log::{error, info};

#[derive(Parser, Debug)]
struct Cli {
    /// Target file to write the OpenAPI Spec
    #[arg(long)]
    openapi: Option<String>,
}

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("(svc-cargo) server startup.");
    dotenv().ok();

    // Expect environment variables
    let config = match config::Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            error!("(config) could not parse config. {}", e);
            panic!();
        }
    };

    // Allow option to only generate the spec file to a given location
    let args = Cli::parse();
    if let Some(target) = args.openapi {
        return rest::generate_openapi_spec(&target);
    }

    // Start Logger
    let log_cfg: &str = config.log_config.as_str();
    if let Err(e) = log4rs::init_file(log_cfg, Default::default()) {
        error!("(logger) could not parse {}. {}", log_cfg, e);
        panic!();
    }

    // Start GRPC Server
    tokio::spawn(grpc::server::server(config.clone()));

    // Start REST API
    rest::server::server(config).await;

    info!("(svc-cargo) successful shutdown.");
    Ok(())
}
