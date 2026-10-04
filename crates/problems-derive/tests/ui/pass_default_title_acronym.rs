use issues::IntoReport;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("failure")]
    HTTPFailure,
}
fn main() {
    let body = serde_json::to_value(Failure::HTTPFailure.into_report().details()).unwrap();
    assert_eq!(body["title"], "Http Failure");
    assert_eq!(body["status"], 500);
}
