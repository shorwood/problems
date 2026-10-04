//! Run with `cargo run -p problems --example poem --features poem`.
//! Request `GET http://127.0.0.1:3000/problem` to see the public conflict document.

use problems::{IntoReport, Report, StatusCode};

#[derive(Debug, thiserror::Error, problems::Problem)]
enum CreateProblem {
    #[error("private duplicate diagnostic: {name}")]
    #[problem(
        type_uri = "urn:example:name-conflict",
        status = StatusCode::CONFLICT,
        title = "Name conflict",
        detail = "The name '{name}' is already in use."
    )]
    NameConflict { name: String },
}

use poem::{Route, Server, handler, listener::TcpListener};

#[handler]
fn problem() -> Report<CreateProblem> {
    CreateProblem::NameConflict {
        name: "example".into(),
    }
    .into_report()
}

fn app() -> Route {
    Route::new().at("/problem", poem::get(problem))
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app())
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use poem::{Endpoint, Request};

    #[tokio::test]
    async fn problem_response() -> Result<(), Box<dyn std::error::Error>> {
        let response = app()
            .call(Request::builder().uri_str("/problem").finish())
            .await?;
        assert_eq!(response.status(), 409);
        assert_eq!(
            response.headers()["content-type"],
            "application/problem+json"
        );
        let body = response.into_body().into_bytes().await?;
        let body: serde_json::Value = serde_json::from_slice(&body)?;
        assert_eq!(body["type"], "urn:example:name-conflict");
        assert_eq!(body["title"], "Name conflict");
        assert_eq!(body["status"], 409);
        assert_eq!(body["detail"], "The name 'example' is already in use.");
        assert_eq!(body.as_object().map(|body| body.len()), Some(4));
        Ok(())
    }
}
