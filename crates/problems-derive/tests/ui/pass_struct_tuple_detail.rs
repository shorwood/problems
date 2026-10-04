use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("storage failed")]
#[problem(type_uri = "urn:test:storage", detail = "Unable to save {1}.")]
struct Storage(#[source] std::io::Error, String);
fn main() {
    assert_eq!(Storage(std::io::Error::other("private"), "document".into()).detail().as_deref(), Some("Unable to save document."));
}
