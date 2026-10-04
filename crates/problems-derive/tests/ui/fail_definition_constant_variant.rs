#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("failure")]
    Conflict,
    #[error("other")]
    #[problem(type_uri = "urn:test:other")]
    CONFLICT,
}
fn main() {}
