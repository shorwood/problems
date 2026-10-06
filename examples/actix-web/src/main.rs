//! Serves the actix-web application on `127.0.0.1:3000`.

use actix_web::{App, HttpServer};
use problems_example_actix_web::app;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(app()))
        .bind("127.0.0.1:3000")?
        .run()
        .await
}
