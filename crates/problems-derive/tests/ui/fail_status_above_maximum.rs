#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("failure")]
    #[problem(type_uri = "urn:test:failure", title = "Failure", status = 1000)]
    Failed,
}
fn main() {}
