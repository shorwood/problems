//! Run with `cargo run -p problems --example warp --features warp`.
//! Request `GET http://127.0.0.1:3000/problem` to see the public conflict document.
//! `GET /recovered` demonstrates application-owned rejection recovery.

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

use warp::{Filter, Reply};

#[derive(Debug)]
struct AppRejection(Report<CreateProblem>);

impl warp::reject::Reject for AppRejection {}

async fn rejected_problem() -> Result<&'static str, warp::Rejection> {
    Err(warp::reject::custom(AppRejection(
        CreateProblem::NameConflict {
            name: "example".into(),
        }
        .into_report(),
    )))
}

async fn recover_problem(
    rejection: warp::Rejection,
) -> Result<warp::reply::Response, warp::Rejection> {
    if let Some(problem) = rejection.find::<AppRejection>() {
        return Ok((&problem.0).into_response());
    }
    Err(rejection)
}

fn recovery_app() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("recovered")
        .and(warp::path::end())
        .and(warp::get())
        .and_then(rejected_problem)
        .recover(recover_problem)
}

fn app() -> impl Filter<Extract = (Report<CreateProblem>,), Error = warp::Rejection> + Clone {
    warp::path("problem")
        .and(warp::path::end())
        .and(warp::get())
        .map(|| {
            CreateProblem::NameConflict {
                name: "example".into(),
            }
            .into_report()
        })
}

#[tokio::main]
async fn main() {
    warp::serve(app().or(recovery_app()))
        .run(([127, 0, 0, 1], 3000))
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recovered_problem_response() -> Result<(), Box<dyn std::error::Error>> {
        let response = warp::test::request()
            .path("/recovered")
            .reply(&recovery_app())
            .await;
        assert_eq!(response.status(), 409);
        assert_eq!(
            response.headers()["content-type"],
            "application/problem+json"
        );
        let body: serde_json::Value = serde_json::from_slice(response.body())?;
        assert_eq!(
            body,
            serde_json::json!({
                "type": "urn:example:name-conflict", "title": "Name conflict",
                "status": 409, "detail": "The name 'example' is already in use."
            })
        );
        Ok(())
    }

    #[tokio::test]
    async fn recovery_preserves_not_found() {
        let response = warp::test::request()
            .path("/missing")
            .reply(&recovery_app())
            .await;
        assert_eq!(response.status(), 404);
        assert_ne!(
            response
                .headers()
                .get("content-type")
                .map(|value| value.as_bytes()),
            Some(b"application/problem+json".as_slice())
        );
    }

    #[tokio::test]
    async fn recovery_preserves_method_rejection() {
        let response = warp::test::request()
            .method("POST")
            .path("/recovered")
            .reply(&recovery_app())
            .await;
        assert_eq!(response.status(), 405);
        assert_ne!(
            response
                .headers()
                .get("content-type")
                .map(|value| value.as_bytes()),
            Some(b"application/problem+json".as_slice())
        );
    }

    #[tokio::test]
    async fn recovery_preserves_custom_rejection() {
        #[derive(Debug)]
        struct Unrelated;
        impl warp::reject::Reject for Unrelated {}
        let rejection = recover_problem(warp::reject::custom(Unrelated))
            .await
            .unwrap_err();
        assert!(rejection.find::<Unrelated>().is_some());
    }

    #[tokio::test]
    async fn problem_response() -> Result<(), Box<dyn std::error::Error>> {
        let response = warp::test::request()
            .method("GET")
            .path("/problem")
            .reply(&app())
            .await;
        assert_eq!(response.status(), 409);
        assert_eq!(
            response.headers()["content-type"],
            "application/problem+json"
        );
        let body = response.body();
        let body: serde_json::Value = serde_json::from_slice(body)?;
        assert_eq!(body["type"], "urn:example:name-conflict");
        assert_eq!(body["title"], "Name conflict");
        assert_eq!(body["status"], 409);
        assert_eq!(body["detail"], "The name 'example' is already in use.");
        assert_eq!(body.as_object().map(|body| body.len()), Some(4));
        Ok(())
    }
}
