use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::{Command, Stdio};

use jsonrpsee::core::client::ClientT;
use jsonrpsee::rpc_params;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;

#[test]
fn must_provide_connection_string() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("tx-sitter")?;

    cmd.arg("daemon");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .stderr(predicate::str::contains("CONNECTION_STRING"));

    Ok(())
}

#[tokio::test]
#[tracing_test::traced_test] // calls tracing::dispatch::set_global_default()
                             // so the call inside cli_batteries will be ignored
async fn app_starts_api() {
    let options = tx_sitter::Options {
        command: tx_sitter::Commands::Daemon,
        connection_string: "sqlite://:memory:".to_owned(),
    };

    assert!(!logs_contain("started"));

    let app = tokio::spawn(async move {
        tx_sitter::app(options).await.expect("app crashed");
    });

    // eventually we should drop tracing_test and write our own subscriber
    // which allows us to await this event instead of polling for it
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(logs_contain("api started"));

    let client = jsonrpsee::ws_client::WsClientBuilder::default()
        .build("ws://localhost:9123")
        .await
        .unwrap();

    assert!(client.is_connected());

    let res: String = client.request("sitter_hi", rpc_params![]).await.unwrap();
    assert_eq!("hi", res);

    cli_batteries::shutdown();
    app.await;
    cli_batteries::reset_shutdown(); // clean up so the next test can run
}
