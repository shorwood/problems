#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test:mapping")]
#[problem(from(bool, |error| match error { true => Self::Invalid }))]
enum Failure {
    #[error("invalid input")]
    #[problem(400)]
    Invalid,
}
fn main() {}
