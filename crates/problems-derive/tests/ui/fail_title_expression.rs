#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("invalid")]
    #[problem(type_uri = "urn:test:invalid", title = concat!("Invalid", " value"))]
    Invalid,
}
fn main() {}
