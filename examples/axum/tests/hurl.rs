//! Server lifecycle glue only: all HTTP assertions live in Hurl fixtures.

use std::net::SocketAddr;

use hurl_test::TestCases;
use miette::{IntoDiagnostic, Result};
use problems_example_axum::app;
use tokio::net::TcpListener;

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
    let listener = TcpListener::bind("127.0.0.1:0").await.into_diagnostic()?;
    let address = listener.local_addr().into_diagnostic()?;
    tokio::spawn(async move { ::axum::serve(listener, app()).await });
    fixtures(address)
}
