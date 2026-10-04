#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("private")]
#[problem(type_uri = "urn:test")]
struct Failure { #[problem(data)] source: std::io::Error }
fn main() {  }
