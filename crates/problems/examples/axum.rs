//! Run with `cargo run -p problems --example axum --features axum`.
//! Request `GET http://127.0.0.1:3000/problem` to see the public conflict document.
//! Compare malformed JSON sent to `POST /json` and `POST /json-problem`.

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

use axum::{
    Json, Router,
    extract::{FromRequest, Request, rejection::JsonRejection},
    response::{IntoResponse, Response},
    routing::{get, post},
};

#[derive(Debug, thiserror::Error, problems::Problem)]
enum RequestProblem {
    #[error("JSON extraction failed: {source}")]
    #[problem(
        type_uri = "urn:example:malformed-json",
        status = 400,
        title = "Malformed JSON",
        detail = "The request body must contain valid JSON."
    )]
    MalformedJson { source: JsonRejection },
}

async fn default_json(payload: Json<serde_json::Value>) -> Json<serde_json::Value> {
    payload
}

struct ProblemJson<T>(T);

impl<T, S> FromRequest<S> for ProblemJson<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(request, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(source @ JsonRejection::JsonSyntaxError(_)) => {
                Err(RequestProblem::MalformedJson { source }
                    .into_report()
                    .into_response())
            }
            Err(rejection) => Err(rejection.into_response()),
        }
    }
}

async fn problem_json(
    ProblemJson(value): ProblemJson<serde_json::Value>,
) -> Json<serde_json::Value> {
    Json(value)
}

async fn problem() -> Result<(), Report<CreateProblem>> {
    Err(CreateProblem::NameConflict {
        name: "example".into(),
    }
    .into_report())
}

fn app() -> Router {
    Router::new()
        .route("/problem", get(problem))
        .route("/json", post(default_json))
        .route("/json-problem", post(problem_json))
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
    async fn default_json_rejection_is_plain_text() -> Result<(), Box<dyn std::error::Error>> {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/json")
                    .header("content-type", "application/json")
                    .body(Body::from("{"))?,
            )
            .await?;
        assert_eq!(response.status(), 400);
        assert_eq!(
            response.headers()["content-type"],
            "text/plain; charset=utf-8"
        );
        Ok(())
    }

    #[tokio::test]
    async fn malformed_json_is_explicitly_classified() -> Result<(), Box<dyn std::error::Error>> {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/json-problem")
                    .header("content-type", "application/json")
                    .body(Body::from("{"))?,
            )
            .await?;
        assert_eq!(response.status(), 400);
        assert_eq!(
            response.headers()["content-type"],
            "application/problem+json"
        );
        let body = to_bytes(response.into_body(), 4096).await?;
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body)?,
            serde_json::json!({
                "type": "urn:example:malformed-json", "title": "Malformed JSON", "status": 400,
                "detail": "The request body must contain valid JSON."
            })
        );
        Ok(())
    }

    #[tokio::test]
    async fn successful_json_extraction_is_unchanged() -> Result<(), Box<dyn std::error::Error>> {
        for path in ["/json", "/json-problem"] {
            let response = app()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(path)
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"name":"example"}"#))?,
                )
                .await?;
            assert_eq!(response.status(), 200);
            assert_eq!(response.headers()["content-type"], "application/json");
            let body = to_bytes(response.into_body(), 4096).await?;
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&body)?,
                serde_json::json!({"name":"example"})
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn other_json_rejections_are_unchanged() -> Result<(), Box<dyn std::error::Error>> {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/json-problem")
                    .body(Body::from("{}"))?,
            )
            .await?;
        assert_eq!(response.status(), 415);
        assert_eq!(
            response.headers()["content-type"],
            "text/plain; charset=utf-8"
        );
        Ok(())
    }

    #[tokio::test]
    async fn body_limit_is_preserved() -> Result<(), Box<dyn std::error::Error>> {
        for path in ["/json", "/json-problem"] {
            let response = app()
                .layer(axum::extract::DefaultBodyLimit::max(8))
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(path)
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"name":"example"}"#))?,
                )
                .await?;
            assert_eq!(response.status(), 413);
            assert_eq!(
                response.headers()["content-type"],
                "text/plain; charset=utf-8"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn missing_route_is_unchanged() -> Result<(), Box<dyn std::error::Error>> {
        let response = app()
            .oneshot(Request::builder().uri("/missing").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), 404);
        assert!(to_bytes(response.into_body(), 4096).await?.is_empty());
        Ok(())
    }

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
