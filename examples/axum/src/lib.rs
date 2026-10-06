//! axum handlers and application routes.

use problems::{IntoReport, Report};

use axum::{
    Json, Router,
    extract::{FromRequest, Request, rejection::JsonRejection},
    response::{IntoResponse, Response},
    routing::{get, post},
};

/**************************************/
/* Public Problem Contract            */
/**************************************/

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

/**************************************/
/* Request Handlers and Routes        */
/**************************************/

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

/// Erases the handler's concrete response type after rendering its report.
async fn opaque_problem() -> impl IntoResponse {
    problem().await.into_response()
}

/// Adds a header while retaining the report's status and body.
async fn header_problem() -> impl IntoResponse {
    (
        [("x-request-id", "example")],
        CreateProblem::NameConflict {
            name: "example".into(),
        }
        .into_report(),
    )
}

/// An outer status changes HTTP metadata while the problem body retains 409.
async fn status_problem() -> impl IntoResponse {
    (
        axum::http::StatusCode::UNAUTHORIZED,
        CreateProblem::NameConflict {
            name: "example".into(),
        }
        .into_report(),
    )
}

/// Builds application routes without binding a listener.
pub fn app() -> Router {
    let mut api = aide::openapi::OpenApi::default();
    let documented = aide::axum::ApiRouter::<()>::new()
        .api_route("/problem", aide::axum::routing::get(problem))
        .finish_api(&mut api);

    documented
        .route("/openapi.json", get(move || async move { Json(api) }))
        .route("/opaque", get(opaque_problem))
        .route("/problem-header", get(header_problem))
        .route("/problem-status", get(status_problem))
        .route("/json", post(default_json))
        .route("/json-problem", post(problem_json))
        .nest(
            "/limited",
            Router::new()
                .route("/json", post(default_json))
                .route("/json-problem", post(problem_json))
                .layer(axum::extract::DefaultBodyLimit::max(8)),
        )
}
