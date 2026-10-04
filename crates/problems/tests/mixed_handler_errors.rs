#![cfg(feature = "axum")]

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::Path,
    http::Request,
    routing::get,
};
use problems::{IntoReport, Report};
use tower::ServiceExt;

#[derive(Debug, thiserror::Error, problems::Problem)]
#[problem(prefix = "urn:test")]
enum AuthProblem {
    #[error("private authentication diagnostic")]
    #[problem(401)]
    Unauthorized,
}

#[derive(Debug, thiserror::Error, problems::Problem)]
#[problem(prefix = "urn:test")]
enum LookupProblem {
    #[error("private lookup diagnostic")]
    #[problem(404)]
    NotFound,
}

#[derive(Debug, thiserror::Error, problems::Problem)]
#[problem(prefix = "urn:test")]
enum StorageProblem {
    #[error("private storage diagnostic")]
    #[problem(503)]
    Unavailable,
}

fn authenticate(case: u8) -> Result<(), AuthProblem> {
    if case == 1 {
        Err(AuthProblem::Unauthorized)
    } else {
        Ok(())
    }
}

fn lookup(case: u8) -> Result<(), LookupProblem> {
    if case == 2 {
        Err(LookupProblem::NotFound)
    } else {
        Ok(())
    }
}

fn store(case: u8) -> Result<(), StorageProblem> {
    if case == 3 {
        Err(StorageProblem::Unavailable)
    } else {
        Ok(())
    }
}

// Existing framework erasure: each original error is first made a report.
#[allow(
    clippy::result_large_err,
    reason = "Exercises Axum's erased error response type"
)]
async fn erased(Path(case): Path<u8>) -> axum::response::Result<&'static str> {
    authenticate(case).map_err(IntoReport::into_report)?;
    lookup(case).map_err(IntoReport::into_report)?;
    store(case).map_err(IntoReport::into_report)?;
    Ok("created")
}

// A boundary enum delegates public metadata to the original problem types.
#[derive(Debug, thiserror::Error, problems::Problem)]
enum HandlerProblem {
    #[error("authentication failed")]
    #[problem(transparent)]
    Auth(#[from] AuthProblem),
    #[error("lookup failed")]
    #[problem(transparent)]
    Lookup(#[from] LookupProblem),
    #[error("storage failed")]
    #[problem(transparent)]
    Storage(#[from] StorageProblem),
}

fn operation(case: u8) -> Result<&'static str, HandlerProblem> {
    authenticate(case)?;
    lookup(case)?;
    store(case)?;
    Ok("created")
}

async fn typed(Path(case): Path<u8>) -> Result<&'static str, Report<HandlerProblem>> {
    operation(case).map_err(IntoReport::into_report)
}

#[tokio::test]
async fn mixed_handlers_preserve_public_responses() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new()
        .route("/erased/{case}", get(erased))
        .route("/typed/{case}", get(typed));
    for style in ["erased", "typed"] {
        for (case, status, suffix, title) in [
            (1, 401, "unauthorized", "Unauthorized"),
            (2, 404, "not-found", "Not Found"),
            (3, 503, "unavailable", "Unavailable"),
        ] {
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/{style}/{case}"))
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), status);
            assert_eq!(
                response.headers()["content-type"],
                "application/problem+json"
            );
            let body = to_bytes(response.into_body(), 4096).await?;
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&body)?,
                serde_json::json!({"type": format!("urn:test:{suffix}"), "title": title, "status": status})
            );
        }
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/{style}/0"))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), 200);
        assert_eq!(to_bytes(response.into_body(), 4096).await?, "created");
    }
    Ok(())
}

#[test]
fn typed_report_preserves_source() {
    use std::error::Error;
    let report = operation(1).unwrap_err().into_report();
    assert!(report.problem().source().unwrap().is::<AuthProblem>());
}

#[cfg(feature = "aide")]
#[test]
fn typed_handler_declares_all_error_statuses() {
    let mut api = aide::openapi::OpenApi::default();
    let _router = aide::axum::ApiRouter::<()>::new()
        .api_route("/typed/{case}", aide::axum::routing::get(typed))
        .finish_api(&mut api);
    let operation = api.paths.as_ref().unwrap().paths["/typed/{case}"]
        .as_item()
        .unwrap()
        .get
        .as_ref()
        .unwrap();
    let responses = &operation.responses.as_ref().unwrap().responses;
    for status in [401, 404, 503] {
        assert!(responses.contains_key(&aide::openapi::StatusCode::Code(status)));
    }
    let api = serde_json::to_value(&api).unwrap();
    let schemas = api["components"]["schemas"].as_object().unwrap();
    let reference = api["paths"]["/typed/{case}"]["get"]["responses"]["401"]["content"]["application/problem+json"]["schema"]["$ref"].as_str().unwrap();
    let name = reference.strip_prefix("#/components/schemas/").unwrap();
    let schema = &schemas[name];
    assert_eq!(schema["properties"]["status"]["minimum"], 100);
    assert_eq!(schema["properties"]["status"]["maximum"], 999);
    for status in ["401", "404", "503"] {
        assert_eq!(
            api["paths"]["/typed/{case}"]["get"]["responses"][status]["content"]["application/problem+json"]
                ["schema"]["$ref"],
            reference
        );
    }
}

#[cfg(feature = "aide")]
#[test]
fn struct_handler_declares_one_problem_response() {
    #[derive(Debug, thiserror::Error, problems::Problem)]
    #[error("name conflict")]
    #[problem(type_uri = "urn:test:name-conflict", status = 409)]
    struct NameConflict;

    async fn handler() -> Result<(), Report<NameConflict>> {
        Err(NameConflict.into_report())
    }

    let mut api = aide::openapi::OpenApi::default();
    let _router = aide::axum::ApiRouter::<()>::new()
        .api_route("/struct", aide::axum::routing::get(handler))
        .finish_api(&mut api);
    let operation = api.paths.as_ref().unwrap().paths["/struct"]
        .as_item()
        .unwrap()
        .get
        .as_ref()
        .unwrap();
    let responses = &operation.responses.as_ref().unwrap().responses;
    let errors: Vec<_> = responses
        .keys()
        .filter(|status| matches!(status, aide::openapi::StatusCode::Code(400..=599)))
        .collect();
    assert_eq!(errors, vec![&aide::openapi::StatusCode::Code(409)]);
    let response = responses[&aide::openapi::StatusCode::Code(409)]
        .as_item()
        .unwrap();
    assert!(response.content.contains_key("application/problem+json"));
}
