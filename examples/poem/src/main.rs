//! Serves the poem application on `127.0.0.1:3000`.

use poem::{Server, listener::TcpListener};
use problems_example_poem::app;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app())
        .await
}
