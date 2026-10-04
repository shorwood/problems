#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("internal error")]
    #[problem(
        type_uri = "urn:test:internal",
        title = "Internal error",
        detail = "Reason: {source}"
    )]
    Internal { r#source: std::io::Error },
}

fn main() {}
