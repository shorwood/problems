#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("failure")]
    #[problem(409)]
    #[problem(status = 409)]
    NameConflict,
}
fn main() {}
