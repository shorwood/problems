#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("invalid")]
    #[problem(transparent)]
    #[problem(status = 409)]
    Invalid(std::io::Error),
}
fn main() {}
