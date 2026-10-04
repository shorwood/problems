use issues::IntoReport;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("failure")]
    NameConflict,
}
fn main() {
    let body = serde_json::to_value(Failure::NameConflict.into_report().as_details()).unwrap();
    assert_eq!(body["type"], "urn:test:name-conflict");
}
