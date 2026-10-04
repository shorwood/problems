#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("invalid")]
    #[problem(transparent)]
    Invalid { source: std::io::Error },
}
fn main() {}
