const STATUS: u16 = 409;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("failure")]
    #[problem(type_uri = "urn:test:failure", title = "Failure", status = STATUS)]
    Failed,
}
fn main() {}
