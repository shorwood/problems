#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("invalid input")]
#[problem(type_uri = "urn:test:invalid")]
struct InvalidInput {
    #[problem(status = 409)]
    value: String,
}
fn main() {}
