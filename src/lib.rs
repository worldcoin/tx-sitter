#![warn(clippy::all)]

pub mod api;
pub mod db;
pub mod proto;
pub mod types;

use clap::{Parser, Subcommand};
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
    Daemon,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("cannot connect to database: {0}")]
    Connect(#[from] anyhow::Error),

    #[error("cannot start server: {0}")]
    StartServer(#[from] api::ServerError),
}

async fn daemon(db: db::Database) -> Result<(), AppError> {
    api::run_server(db).await.map_err(AppError::StartServer)?;

    cli_batteries::await_shutdown().await;

    Ok(())
}

pub async fn app(options: Options) -> Result<(), AppError> {
    let database = db::Database::new(options.database)
        .await
        .map_err(AppError::Connect)?;

    match options.command {
        Commands::Daemon => daemon(database).await?,
    }
    Ok(())
}
