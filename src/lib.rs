#![warn(clippy::all)]

pub mod api;
pub mod db;
pub mod proto;
pub mod types;

use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use thiserror::Error;
use tracing::error;

#[derive(Parser)]
pub struct Options {
    #[command(subcommand)]
    pub command: Commands,

    #[clap(flatten)]
    pub database: db::Options,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    // start the api, connect to upstreams, the primary mode
    Daemon {
        #[clap(long, env, default_value = "127.0.0.1:9123")]
        api_address: SocketAddr,
    },
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("cannot connect to database: {0}")]
    Connect(#[from] anyhow::Error),

    #[error("cannot start server: {0}")]
    StartServer(#[from] api::ServerError),
}

async fn daemon(api_address: SocketAddr, db: db::Database) -> Result<(), AppError> {
    api::run_server(api_address, db)
        .await
        .map_err(AppError::StartServer)?;

    cli_batteries::await_shutdown().await;

    Ok(())
}

pub async fn app(options: Options) -> Result<(), AppError> {
    let database = db::Database::new(options.database)
        .await
        .map_err(AppError::Connect)?;

    match options.command {
        Commands::Daemon { api_address } => daemon(api_address, database).await?,
    }
    Ok(())
}
