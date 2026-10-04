#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("private diagnostic")]
    #[problem(detail = "{0}")]
    Conflict(#[source] std::io::Error),
}
fn main() {  }
