use std::net::SocketAddr;

use jsonrpsee::server::{RpcModule, ServerBuilder};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("bad address")]
    BadAddress(#[from] std::net::AddrParseError),

    #[error("failed to bind")]
    BindError(#[from] jsonrpsee::core::Error),
}

pub async fn run_server() -> Result<(), ServerError> {
    let addr = "127.0.0.1:9123"
        .parse::<SocketAddr>()
        .map_err(ServerError::BadAddress)?;

    let server = ServerBuilder::default()
        .build(addr)
        .await
        .map_err(ServerError::BindError)?;

    let mut module = RpcModule::new(());
    module
        .register_method("sitter_hi", |_, _| Ok("hi"))
        .unwrap();

    let handle = server.start(module)?;
    info!(addr="127.0.0.1:9123", "api started");

    // - the server will shutdown once this handle is dropped
    // - handle.stopped() blocks until someone calls handle.stop()
    // so this spawn keeps the server running until the tokio
    // runtime is dropped
    tokio::spawn(handle.stopped());

    Ok(())
}
