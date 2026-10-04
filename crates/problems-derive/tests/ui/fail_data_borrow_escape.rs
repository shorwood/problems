#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("private")]
#[problem(type_uri = "urn:test")]
struct Failure {
    #[problem(data)]
    value: String,
}
fn main() {
    let details;
    {
        let report = issues::Report::new(Failure { value: "x".into() });
        details = report.as_details();
    }
    let _ = serde_json::to_value(details);
}
