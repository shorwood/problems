use problems::{IntoReport, Report};
use std::error::Error;

#[derive(Debug, thiserror::Error, problems::Problem)]
#[problem(prefix = "urn:test")]
pub enum Failure<'a> {
    #[error("private: {source}")]
    #[problem(status = 409, title = "Conflict", detail = "Name: {name}")]
    Conflict {
        name: &'a str,
        source: std::io::Error,
    },
}

pub fn report(name: &str) -> Report<Failure<'_>> {
    Failure::Conflict {
        name,
        source: std::io::Error::other("private source"),
    }
    .into_report()
}

pub fn assert_diagnostics(report: &Report<Failure<'_>>) {
    assert!(report.problem().source().unwrap().is::<std::io::Error>());
    assert!(report.problem().to_string().contains("private source"));
}

pub fn assert_body(body: serde_json::Value) {
    assert_eq!(
        body,
        serde_json::json!({
            "type": "urn:test:conflict", "title": "Conflict",
            "status": 409, "detail": "Name: example"
        })
    );
}
