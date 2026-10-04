use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("failure")]
    #[problem(type_uri = "urn:test:failure", title = "Failure", status = issues::StatusCode::CONFLICT)]
    Failed,
}
fn main() {
    assert_eq!(
        Failure::Failed.definition().status,
        issues::StatusCode::CONFLICT
    );
}
