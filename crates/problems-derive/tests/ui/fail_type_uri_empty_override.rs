#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("failure")]
    #[problem(type_uri = "")]
    NameConflict,
}
fn main() {}
