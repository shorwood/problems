//! Run with `cargo run -p problems --example rocket --features rocket`.
//! Request `GET http://127.0.0.1:3000/problem` to see the public conflict document.

use problems::{IntoReport, Report};

#[derive(Debug, thiserror::Error, problems::Problem)]
enum CreateProblem {
    #[error("private duplicate diagnostic: {name}")]
    #[problem(
        type_uri = "urn:example:name-conflict",
        status = 409,
        title = "Name conflict",
        detail = "The name '{name}' is already in use."
    )]
    NameConflict { name: String },
}

#[rocket::get("/problem")]
fn problem() -> Result<(), Report<CreateProblem>> {
    Err(CreateProblem::NameConflict {
        name: "example".into(),
    }
    .into_report())
}

fn app() -> rocket::Rocket<rocket::Build> {
    rocket::build().mount("/", rocket::routes![problem])
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[rocket::async_test]
    async fn problem_response() -> Result<(), Box<dyn std::error::Error>> {
        let client = rocket::local::asynchronous::Client::tracked(app()).await?;
        let response = client.get("/problem").dispatch().await;
        assert_eq!(response.status().code, 409);
        assert_eq!(
            response.headers().get_one("content-type"),
            Some("application/problem+json")
        );
        let body = response.into_bytes().await.ok_or("missing response body")?;
        let body: serde_json::Value = serde_json::from_slice(&body)?;
        assert_eq!(body["type"], "urn:example:name-conflict");
        assert_eq!(body["title"], "Name conflict");
        assert_eq!(body["status"], 409);
        assert_eq!(body["detail"], "The name 'example' is already in use.");
        assert_eq!(body.as_object().map(|body| body.len()), Some(4));
        Ok(())
    }
}
