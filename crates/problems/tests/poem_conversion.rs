#![cfg(all(feature = "poem", feature = "derive"))]

use problems::{IntoReport, StatusCode};
use std::cell::Cell;

#[derive(Debug, thiserror::Error, problems::Problem)]
enum BorrowedProblem<'a> {
    #[error("private: {name}")]
    #[problem(type_uri = "urn:test:borrowed", title = "Borrowed problem")]
    Failed { name: &'a str, diagnostic: Cell<u8> },
}

#[tokio::test]
async fn conversion_accepts_borrowed_non_sync_errors() {
    let name = String::from("password=SECRET");
    let report = BorrowedProblem::Failed {
        name: &name,
        diagnostic: Cell::new(1),
    }
    .into_report();
    let error: poem::Error = report.into();
    assert!(error.is_from_response());
    assert!(!error.has_source());
    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response.headers()["content-type"],
        "application/problem+json"
    );
    let body: serde_json::Value = response.into_body().into_json().await.unwrap();
    assert_eq!(body["title"], "Borrowed problem");
    assert_eq!(body["status"], 500);
    assert_eq!(body.as_object().unwrap().len(), 3);
}
