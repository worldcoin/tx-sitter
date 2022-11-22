#![warn(clippy::all)]

mod api;
mod db;

use clap::{Parser, Subcommand};
use cli_batteries::version;
use thiserror::Error;
use tracing::info;

#[derive(Parser)]
struct Options {
    #[command(subcommand)]
    command: Commands,

    #[arg(env = "SITTER_CONNECTION_STRING")]
    connection_string: String,
}

#[derive(Debug, Subcommand)]
enum Commands {
    // start the api, connect to upstreams, the primary mode
    Daemon,
}

#[derive(Error, Debug)]
enum AppError {
    #[error("cannot connect to database")]
    Connect(#[from] sqlx::Error),

    #[error("cannot start server")]
    StartServer(#[from] api::ServerError),
}

async fn daemon(_db: db::Database) -> Result<(), AppError> {
    info!("starting daemon");
    api::run_server().await.map_err(AppError::StartServer)?;

    cli_batteries::await_shutdown().await;

    Ok(())
}

async fn app(options: Options) -> Result<(), AppError> {
    //
    // confirm we can use the requested database
    let database = db::Database::connect(&options.connection_string)
        .await
        .map_err(AppError::Connect)?;

    match options.command {
        Commands::Daemon => daemon(database).await?,
    }
    Ok(())
}

fn main() {
    cli_batteries::run(version!(), app);
}
