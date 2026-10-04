//! Run with `cargo run -p problems --example warp --features warp`.
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

use warp::Filter;

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
    warp::serve(app()).run(([127, 0, 0, 1], 3000)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

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
