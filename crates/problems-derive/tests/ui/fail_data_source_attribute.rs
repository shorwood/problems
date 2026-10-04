#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("private")]
#[problem(type_uri = "urn:test")]
struct Failure {
    #[source]
    #[problem(data)]
    diagnostic: std::io::Error,
}
fn main() {}
