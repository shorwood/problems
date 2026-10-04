#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("failure")]
    #[problem(status = 409)]
    #[problem(409)]
    NameConflict,
}
fn main() {}
