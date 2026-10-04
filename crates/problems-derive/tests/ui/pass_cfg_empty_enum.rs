use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Disabled {
    #[cfg(any())]
    #[error("disabled")]
    #[problem(type_uri = "urn:test:disabled", title = "Disabled")]
    Removed,
}
fn main() {
    assert!(Disabled::definitions().is_empty());
}
