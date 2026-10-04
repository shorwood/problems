#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure { #[error(transparent)] #[problem(500)] Wrapped(#[from] #[problem(data = "source")] std::io::Error) }
fn main() {  }
