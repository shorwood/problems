#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("failure")]
    #[problem("409")]
    NameConflict,
}
fn main() {
    use issues::Problem;
    assert_eq!(Failure::NameConflict.detail().as_deref(), Some("409"));
}
