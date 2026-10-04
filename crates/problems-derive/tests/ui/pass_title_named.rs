use issues::IntoReport;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("invalid {name}")]
    #[problem(
        type_uri = "urn:test:invalid",
        title = "Expected {name}",
        detail = "Invalid {name}"
    )]
    Invalid { name: String },
}
fn main() {
    let report = Failure::Invalid {
        name: "public".into(),
    }
    .into_report();
    let body = serde_json::to_value(report.details()).unwrap();
    assert_eq!(body["title"], "Expected {name}");
    assert_eq!(body["detail"], "Invalid public");
}
