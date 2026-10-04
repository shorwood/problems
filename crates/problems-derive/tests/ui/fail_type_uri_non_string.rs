#[derive(Debug, thiserror::Error, problems_derive::Problem)]

enum Failure {
    #[error("failure")]
    #[problem(type_uri = 409)]
    NameConflict,
}
fn main() {}
