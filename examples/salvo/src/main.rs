//! Serves the salvo application on `127.0.0.1:3000`.

use problems_example_salvo::app;
use salvo::{Server, prelude::TcpListener};

#[tokio::main]
async fn main() {
    use salvo::Listener;
    let acceptor = TcpListener::new("127.0.0.1:3000").bind().await;
    Server::new(acceptor).serve(app()).await;
}
