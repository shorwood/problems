use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("private diagnostic")]
    #[problem(detail = "{0:04x}")]
    Conflict(u32),
}
fn main() { assert_eq!(Failure::Conflict(15).detail().as_deref(), Some("000f")); }
