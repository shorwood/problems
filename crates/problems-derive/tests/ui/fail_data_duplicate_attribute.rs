#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("private")]
#[problem(type_uri = "urn:test")]
struct Failure {
    #[problem(data)]
    #[problem(data)]
    value: String,
}
fn main() {}
