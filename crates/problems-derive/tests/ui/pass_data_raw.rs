#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("private")]
#[problem(type_uri = "urn:test")]
struct Failure {
    #[problem(data)]
    r#type: String,
}
fn main() {
    let report = issues::Report::new(Failure { r#type: "x".into() });
    assert_eq!(
        serde_json::to_value(report.as_details()).unwrap()["data"]["type"],
        "x"
    );
}
