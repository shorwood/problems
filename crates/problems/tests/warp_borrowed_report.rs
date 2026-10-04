#![cfg(feature = "warp")]

use problems::{IntoReport, Report};
use warp::{Filter, Reply};

#[derive(Debug, thiserror::Error, problems::Problem)]
#[problem(prefix = "urn:test")]
enum Failure<'a> {
    #[error("private error: {source}")]
    #[problem(status = 500, detail = "Unable to read {name}.")]
    Read {
        name: &'a str,
        source: std::io::Error,
    },
}

fn render(name: &str) -> warp::reply::Response {
    use std::error::Error;
    let report: Report<Failure<'_>> = Failure::Read {
        name,
        source: std::io::Error::other("private source"),
    }
    .into_report();
    let response = (&report).into_response();
    assert!(report.problem().source().unwrap().is::<std::io::Error>());
    response
}

#[tokio::test]
async fn borrowed_report_accepts_non_clone_error_and_returns_owned_response()
-> Result<(), Box<dyn std::error::Error>> {
    let route = warp::any().map(|| {
        let name = String::from("example");
        render(&name)
    });
    let response = warp::test::request().reply(&route).await;
    assert_eq!(response.status(), 500);
    assert_eq!(
        response.headers()["content-type"],
        "application/problem+json"
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(response.body())?,
        serde_json::json!({
            "type": "urn:test:read", "title": "Read", "status": 500,
            "detail": "Unable to read example."
        })
    );
    Ok(())
}
