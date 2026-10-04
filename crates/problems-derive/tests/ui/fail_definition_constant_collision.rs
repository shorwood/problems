#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("first")]
    #[problem(type_uri = "urn:test:first")]
    HttpError,
    #[error("second")]
    #[problem(type_uri = "urn:test:second")]
    HTTPError,
}
fn main() {}
