#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("failure")]
    #[problem(type_uri = "urn:test:failure", title = "Failure", status = issues::StatusCode::CONFLICT, status = 409)]
    Failed,
}
fn main() {}
