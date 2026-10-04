#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("named")]
    #[problem(detail = "{missing}")]
    Named {
        source: std::io::Error,
        value: String,
    },
    #[error("tuple")]
    #[problem(detail = "{2}")]
    Tuple(#[source] std::io::Error, String),
    #[error("source only")]
    #[problem(detail = "{missing}")]
    SourceOnly { source: std::io::Error },
    #[error("unit")]
    #[problem(detail = "{missing}")]
    Unit,
}

fn main() {}
