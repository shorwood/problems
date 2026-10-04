#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("invalid data")]
    #[problem(type_uri = "urn:test:data")]
    InvalidData {
        #[problem(data = "")]
        value: String,
    },
    #[error("missing URI")]
    MissingUri,
    #[error("invalid status")]
    #[problem(type_uri = "urn:test:status", status = "409")]
    InvalidStatus,
    #[error("unknown detail field")]
    #[problem(type_uri = "urn:test:detail", detail = "{missing}")]
    InvalidDetail {
        #[problem(data = "")]
        value: String,
    },
}

fn main() {}
