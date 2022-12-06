use std::net::SocketAddr;
use std::time::Duration;

mod proto {
    tonic::include_proto!("sitter_v1");
}

#[tokio::test]
#[tracing_test::traced_test] // calls tracing::dispatch::set_global_default()
                             // so the call inside cli_batteries will be ignored
async fn app_starts_api() {
    let options = tx_sitter::Options {
        command: tx_sitter::Commands::Daemon {
            api_address: SocketAddr::from(([127, 0, 0, 1], 9123)),
        },
        database: tx_sitter::db::Options {
            database: url::Url::parse("sqlite::memory:").unwrap(),
            database_migrate: true,
            database_max_connections: 10,
        },
    };

    assert!(!logs_contain("started"));

    let app = tokio::spawn(async move {
        tx_sitter::app(options).await.expect("app crashed");
    });

    // eventually we should drop tracing_test and write our own subscriber
    // which allows us to await this event instead of polling for it
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(logs_contain("api started"));

    use proto::sitter_client::SitterClient;
    let mut client = SitterClient::connect("http://localhost:9123")
        .await
        .unwrap();

    let request = tonic::Request::new(proto::StatusRequest {});
    let _response = client.status(request).await.unwrap();

    cli_batteries::shutdown();
    app.await.unwrap();
    cli_batteries::reset_shutdown(); // clean up so the next test can run
}
