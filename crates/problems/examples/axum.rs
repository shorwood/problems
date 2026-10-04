//! Run with `cargo run -p problems --example axum --features axum`.
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

use axum::{Router, routing::get};

async fn problem() -> Result<(), Report<CreateProblem>> {
    Err(CreateProblem::NameConflict {
        name: "example".into(),
    }
    .into_report())
}

fn app() -> Router {
    Router::new().route("/problem", get(problem))
}

#[cfg(test)]
async fn opaque_problem() -> impl axum::response::IntoResponse {
    use axum::response::IntoResponse;
    problem().await.into_response()
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn opaque_response() -> Result<(), Box<dyn std::error::Error>> {
        let router = Router::new().route("/opaque", get(opaque_problem));
        let response = router
            .oneshot(Request::builder().uri("/opaque").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), 409);
        assert_eq!(
            response.headers()["content-type"],
            "application/problem+json"
        );
        let body = to_bytes(response.into_body(), 4096).await?;
        let body: serde_json::Value = serde_json::from_slice(&body)?;
        assert_eq!(body["type"], "urn:example:name-conflict");
        assert_eq!(body["status"], 409);
        Ok(())
    }

    #[cfg(feature = "aide")]
    #[test]
    fn concrete_report_infers_conflict() {
        let mut api = aide::openapi::OpenApi::default();
        let _router = aide::axum::ApiRouter::<()>::new()
            .api_route("/problem", aide::axum::routing::get(problem))
            .finish_api(&mut api);
        let operation = api.paths.as_ref().unwrap().paths["/problem"]
            .as_item()
            .unwrap()
            .get
            .as_ref()
            .unwrap();
        let response = operation.responses.as_ref().unwrap().responses
            [&aide::openapi::StatusCode::Code(409)]
            .as_item()
            .unwrap();
        assert!(response.description.contains("urn:example:name-conflict"));
        assert!(response.content.contains_key("application/problem+json"));
    }

    #[tokio::test]
    async fn problem_response() -> Result<(), Box<dyn std::error::Error>> {
        let response = app()
            .oneshot(Request::builder().uri("/problem").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), 409);
        assert_eq!(
            response.headers()["content-type"],
            "application/problem+json"
        );
        let body = to_bytes(response.into_body(), 4096).await?;
        let body: serde_json::Value = serde_json::from_slice(&body)?;
        assert_eq!(body["type"], "urn:example:name-conflict");
        assert_eq!(body["title"], "Name conflict");
        assert_eq!(body["status"], 409);
        assert_eq!(body["detail"], "The name 'example' is already in use.");
        assert_eq!(body.as_object().map(|body| body.len()), Some(4));
        Ok(())
    }
}
