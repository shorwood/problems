use issues::{IntoReport, Problem};
use std::error::Error;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("storage failed")]
    Storage(#[from] std::io::Error),
}
fn main() {
    let report = Failure::from(std::io::Error::other("private source")).into_report();
    assert!(report.problem().source().unwrap().is::<std::io::Error>());
    assert_eq!(report.problem().definition(), &Failure::STORAGE);
    assert_eq!(Failure::definitions().next().unwrap(), &Failure::STORAGE);
    assert_eq!(serde_json::to_value(report.details()).unwrap(), serde_json::json!({
        "type": "urn:test:storage", "title": "Storage", "status": 500
    }));
}
