#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = 409)]
enum Failure {
    #[error("failure")]
    NameConflict,
}
fn main() {}
