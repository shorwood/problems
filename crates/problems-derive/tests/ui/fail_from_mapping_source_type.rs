#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test:mapping")]
#[problem(from(MissingError, |_error| Self::Invalid))]
enum Failure {
    #[error("invalid input")]
    #[problem(400)]
    Invalid,
}
fn main() {}
