#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("invalid")]
    #[problem(transparent)]
    Invalid(std::io::Error, std::io::Error),
}
fn main() {}
