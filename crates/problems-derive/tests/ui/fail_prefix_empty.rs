#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "")]
enum Failure {
    #[error("failure")]
    NameConflict,
}
fn main() {}
