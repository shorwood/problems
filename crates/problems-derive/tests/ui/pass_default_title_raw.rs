#![allow(non_camel_case_types)]

use issues::IntoReport;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("failure")]
    r#type,
}
fn main() {
    let body = serde_json::to_value(Failure::r#type.into_report().details()).unwrap();
    assert_eq!(body["title"], "Type");
    assert_eq!(body["status"], 500);
}
