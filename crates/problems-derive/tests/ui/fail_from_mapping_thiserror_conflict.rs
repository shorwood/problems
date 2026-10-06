#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test:mapping")]
#[problem(from(std::io::Error, |source| Self::Storage { source }))]
enum Failure {
    #[error("storage operation failed")]
    Storage {
        #[from]
        source: std::io::Error,
    },
}
fn main() {}
