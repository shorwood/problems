use issues::IntoReport;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("private diagnostic")]
    #[problem(detail = "Use {{another}} name.")]
    Conflict(String),
}
fn main() {
    let details = Failure::Conflict("private field".into())
        .into_report()
        .into_details();
    assert_eq!(details.detail(), Some("Use {another} name."));
}
