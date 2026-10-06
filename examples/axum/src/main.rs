//! Serves the axum application on `127.0.0.1:3000`.

use problems_example_axum::app;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app()).await
}
