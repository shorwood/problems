use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("private diagnostic")]
    #[problem(detail = "Name: {1}")]
    Conflict(#[source] std::io::Error, String),
}
fn main() { assert_eq!(Failure::Conflict(std::io::Error::other("private"), "taken".into()).detail().as_deref(), Some("Name: taken")); }
