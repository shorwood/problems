use issues::IntoReport;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("failure")]
    #[problem(status = 409)]
    NameConflict,
}
fn main() {
    let body = serde_json::to_value(Failure::NameConflict.into_report().details()).unwrap();
    assert_eq!(
        body,
        serde_json::json!({"type":"urn:test:name-conflict", "title":"Name Conflict", "status":409})
    );
}
