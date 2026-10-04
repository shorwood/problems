//! Run with `cargo run -p problems --example actix-web --features actix-web`.
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

use actix_web::{App, HttpServer, web};

async fn problem() -> Result<&'static str, Report<CreateProblem>> {
    Err(CreateProblem::NameConflict {
        name: "example".into(),
    }
    .into_report())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/problem", web::get().to(problem)))
        .bind("127.0.0.1:3000")?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode as ActixStatusCode, test};

    #[actix_web::test]
    async fn problem_response() -> Result<(), Box<dyn std::error::Error>> {
        let app = test::init_service(App::new().route("/problem", web::get().to(problem))).await;
        let response =
            test::call_service(&app, test::TestRequest::get().uri("/problem").to_request()).await;
        assert_eq!(response.status(), ActixStatusCode::CONFLICT);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("application/problem+json")
        );
        let body = test::read_body(response).await;
        let body: serde_json::Value = serde_json::from_slice(&body)?;
        assert_eq!(body["type"], "urn:example:name-conflict");
        assert_eq!(body["title"], "Name conflict");
        assert_eq!(body["status"], 409);
        assert_eq!(body["detail"], "The name 'example' is already in use.");
        assert_eq!(body.as_object().map(|body| body.len()), Some(4));
        Ok(())
    }
}
