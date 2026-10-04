use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("private diagnostic")]
    #[problem(detail = "Name: {0}")]
    Conflict(String),
}
fn main() {
    assert_eq!(
        Failure::Conflict("taken".into()).detail().as_deref(),
        Some("Name: taken")
    );
}
