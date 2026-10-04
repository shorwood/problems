#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("invalid")]
    #[problem(transparent)]
    Invalid,
}
fn main() {}
