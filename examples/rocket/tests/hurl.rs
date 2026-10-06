//! Server lifecycle glue only: all HTTP assertions live in Hurl fixtures.

use std::net::SocketAddr;

use hurl_test::TestCases;
use miette::{IntoDiagnostic, Result};
use problems_example_rocket::app;

fn fixtures(address: SocketAddr) -> Result<()> {
    TestCases::new()
        .variable("base_url", format!("http://{address}"))
        .ignore("tests/hurl/templates/**")?
        .include("tests/hurl/**/*.hurl")?
        .run()?;
    Ok(())
}

// Hurl blocks the test thread; the server runs on another worker.
// The test runtime owns the server task and stops it if a fixture fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui() -> Result<()> {
    // Rocket binds its own listener. Liftoff supplies the assigned port after bind.
    let (ready, address) = tokio::sync::oneshot::channel();
    let config = ::rocket::Config {
        address: std::net::Ipv4Addr::LOCALHOST.into(),
        port: 0,
        ..::rocket::Config::default()
    };
    let app = app()
        .configure(config)
        .attach(::rocket::fairing::AdHoc::on_liftoff(
            "Hurl address",
            |rocket| {
                Box::pin(async move {
                    let config = rocket.config();
                    let _ = ready.send(SocketAddr::new(config.address, config.port));
                })
            },
        ));
    tokio::spawn(async move { app.launch().await });
    fixtures(address.await.into_diagnostic()?)
}
