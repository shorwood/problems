#[derive(Debug, thiserror::Error, problems_derive::Problem)]

enum Failure {
    #[error("failure")]
    #[problem(type_uri = "urn:test:a")]
    #[problem(type_uri = "urn:test:b")]
    NameConflict,
}
fn main() {}
