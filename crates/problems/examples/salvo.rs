//! Run with `cargo run -p problems --example salvo --features salvo`.
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

use salvo::{Router, Server, handler, prelude::TcpListener};

#[handler]
fn problem() -> Report<CreateProblem> {
    CreateProblem::NameConflict {
        name: "example".into(),
    }
    .into_report()
}

fn app() -> Router {
    Router::with_path("problem").get(problem)
}

#[tokio::main]
async fn main() {
    use salvo::Listener;
    let acceptor = TcpListener::new("127.0.0.1:3000").bind().await;
    Server::new(acceptor).serve(app()).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use salvo::test::{ResponseExt, TestClient};

    #[tokio::test]
    async fn problem_response() -> Result<(), Box<dyn std::error::Error>> {
        let mut response = TestClient::get("http://localhost/problem")
            .send(app())
            .await;
        assert_eq!(
            response.status_code.map(|status| status.as_u16()),
            Some(409)
        );
        assert_eq!(
            response.headers()["content-type"],
            "application/problem+json"
        );
        let body = response.take_string().await?.into_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body)?;
        assert_eq!(body["type"], "urn:example:name-conflict");
        assert_eq!(body["title"], "Name conflict");
        assert_eq!(body["status"], 409);
        assert_eq!(body["detail"], "The name 'example' is already in use.");
        assert_eq!(body.as_object().map(|body| body.len()), Some(4));
        Ok(())
    }
}
