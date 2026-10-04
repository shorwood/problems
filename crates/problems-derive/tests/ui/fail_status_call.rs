const fn status() -> issues::StatusCode {
    issues::StatusCode::CONFLICT
}
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("failure")]
    #[problem(type_uri = "urn:test:failure", title = "Failure", status = status())]
    Failed,
}
fn main() {}
