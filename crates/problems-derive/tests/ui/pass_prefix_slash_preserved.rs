use issues::IntoReport;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "https://example.com/problems/")]
enum Failure {
    #[error("failure")]
    NameConflict,
}
fn main() {
    let body = serde_json::to_value(Failure::NameConflict.into_report().as_details()).unwrap();
    assert_eq!(body["type"], "https://example.com/problems/name-conflict");
}
