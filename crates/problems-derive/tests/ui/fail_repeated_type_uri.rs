#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("first")]
    #[problem(type_uri = "urn:test:repeated")]
    First,
    #[error("second")]
    #[problem(type_uri = "urn:test:repeated")]
    Second,
    #[error("third")]
    #[problem(type_uri = "urn:test:repeated")]
    Third,
}

fn main() {}
