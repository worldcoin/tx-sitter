use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::{
    Command,
    Stdio,
};

use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::rpc_params;

#[test]
fn must_provide_connection_string() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("tx-sitter")?;

    cmd.arg("daemon");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required arguments were not provided"))
        .stderr(predicate::str::contains("CONNECTION_STRING"));

    Ok(())
}

async fn read_until_match<R: tokio::io::AsyncRead + std::marker::Unpin>(read: R, target: &str) {
    let mut bufread = tokio::io::BufReader::new(read);
    loop {
        let mut data = String::new();
        bufread.read_line(&mut data).await.unwrap();
        println!("data {data}");
        if data.contains(target) {
            break
        }
    }
}

#[tokio::test]
async fn starts_api() {
    let cmd_path = assert_cmd::cargo::cargo_bin("tx-sitter");
    let mut child = tokio::process::Command::new(cmd_path)
        .arg(":memory:")
        .arg("daemon")
        .stderr(Stdio::piped())
        .kill_on_drop(true)  // caveat: tokio promises "best-effort" to reap this zombie
        .spawn().expect("failed to spawn tx-sitter");

    let stderr = child.stderr.take().unwrap();
    if let Err(_) = tokio::time::timeout(
        Duration::from_secs(1),
        read_until_match(stderr, "api started"),
    ).await {
        panic!("api did not start in time");
    }

    let client = jsonrpsee::ws_client::WsClientBuilder::default()
        .build("ws://localhost:9123")
        .await
        .unwrap();

    assert_eq!(true, client.is_connected());

    let res: String = client.request("sitter_hi", rpc_params![]).await.unwrap();
    assert_eq!("hi", res);

    child.kill().await.unwrap();
    child.wait().await.unwrap();
}
