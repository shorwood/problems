use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("failure")]
    #[problem(type_uri = "urn:test:failure", title = "Failure", status = 999)]
    Failed,
}
fn main() {
    assert_eq!(Failure::Failed.definition().status.as_u16(), 999);
}
