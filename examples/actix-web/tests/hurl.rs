//! Server lifecycle glue only: all HTTP assertions live in Hurl fixtures.

use std::net::SocketAddr;

use hurl_test::TestCases;
use miette::{IntoDiagnostic, Result};
use problems_example_actix_web::app;

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
#[::actix_web::test]
async fn ui() -> Result<()> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").into_diagnostic()?;
    let address = listener.local_addr().into_diagnostic()?;
    let server = ::actix_web::HttpServer::new(|| ::actix_web::App::new().service(app()))
        .listen(listener)
        .into_diagnostic()?
        .run();
    let handle = server.handle();
    ::actix_web::rt::spawn(server);
    let result = tokio::task::spawn_blocking(move || fixtures(address))
        .await
        .into_diagnostic()?;
    handle.stop(true).await;
    result
}
