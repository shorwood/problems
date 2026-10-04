use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("private diagnostic")]
    #[problem(detail = "{1}/{0}/{1}")]
    Conflict(u32, u32),
}
fn main() {
    assert_eq!(Failure::Conflict(1, 2).detail().as_deref(), Some("2/1/2"));
}
