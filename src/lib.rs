#![warn(clippy::all)]

mod api;
mod db;

use clap::{Parser, Subcommand};
use thiserror::Error;
use tracing::error;

#[derive(Parser)]
pub struct Options {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(env = "SITTER_CONNECTION_STRING")]
    pub connection_string: String,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    // start the api, connect to upstreams, the primary mode
    Daemon,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("cannot connect to database")]
    Connect(#[from] sqlx::Error),

    #[error("cannot start server")]
    StartServer(#[from] api::ServerError),
}

async fn daemon(_db: db::Database) -> Result<(), AppError> {
    api::run_server().await.map_err(AppError::StartServer)?;

    cli_batteries::await_shutdown().await;

    Ok(())
}

pub async fn app(options: Options) -> Result<(), AppError> {
    let database = db::Database::connect(&options.connection_string)
        .await
        .map_err(AppError::Connect)?;

    use db::MigrationStatus::*;
    match database.migration_status().await? {
        Dirty => {
            error!("database is is an inconsistent migration state");
            return Ok(());
        }
        Empty | Behind => {
            database.migrate().await?;
        }
        Current => {}
        Ahead => {
            error!("tx-sitter must be updated to use this database");
            return Ok(());
        }
    };

    match options.command {
        Commands::Daemon => daemon(database).await?,
    }
    Ok(())
}
