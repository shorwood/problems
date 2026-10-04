#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("storage failed")]
#[problem(type_uri = "urn:test:storage", detail = "{source}")]
struct Storage {
    source: std::io::Error,
}
fn main() {}
