//! Serves the rocket application on `127.0.0.1:3000`.

use problems_example_rocket::app;

#[rocket::main]
async fn main() -> Result<(), Box<rocket::Error>> {
    let config = rocket::Config {
        address: std::net::Ipv4Addr::LOCALHOST.into(),
        port: 3000,
        ..rocket::Config::default()
    };
    app().configure(config).launch().await?;
    Ok(())
}
