#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error(transparent)]
#[problem(transparent)]
struct Storage(std::io::Error);
fn main() {}
