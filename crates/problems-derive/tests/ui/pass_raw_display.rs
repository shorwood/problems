use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("invalid value")]
    #[problem(type_uri = "urn:test:invalid", title = "Invalid", detail = "{type}")]
    Invalid { r#type: String },
}
fn main() {
    let value = "hello".to_owned();
    let expected = format!("{type}", r#type = value);
    let failure = Failure::Invalid { r#type: value };
    assert_eq!(failure.detail(), Some(expected));
}
