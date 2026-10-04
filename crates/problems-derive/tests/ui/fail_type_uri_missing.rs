#[derive(Debug, thiserror::Error, problems_derive::Problem)]

enum Failure {
    #[error("failure")]
    NameConflict,
}
fn main() {}
