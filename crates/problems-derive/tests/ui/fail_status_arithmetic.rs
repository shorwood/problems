const STATUS: u16 = 200;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("failure")]
    #[problem(type_uri = "urn:test:failure", title = "Failure", status = STATUS + STATUS)]
    Failed,
}
fn main() {}
