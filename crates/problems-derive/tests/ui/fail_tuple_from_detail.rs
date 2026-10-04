#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("failure")]
    #[problem(detail = "Failure: {0}")]
    Storage(#[from] std::io::Error),
}
fn main() {}
