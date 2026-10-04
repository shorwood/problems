#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("malformed")]
    #[problem(type_uri = 123)]
    Malformed {
        #[problem(data = "")]
        value: String,
    },
    #[error("empty title")]
    #[problem(type_uri = "urn:test:title", title = "")]
    EmptyTitle,
    #[error("empty URI")]
    #[problem(type_uri = "")]
    EmptyUri,
}

fn main() {}
