#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
#[problem(prefix = "urn:other")]
enum Failure {
    #[error("failure")]
    NameConflict,
}
fn main() {}
