#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("source")]
    Source {
        #[problem(data)]
        source: std::io::Error,
    },
    #[error("empty name")]
    EmptyName {
        #[problem(data = "")]
        value: String,
    },
    #[error("duplicate name")]
    DuplicateName {
        #[problem(data = "value")]
        first: String,
        #[problem(data = "value")]
        second: String,
    },
}

fn main() {}
