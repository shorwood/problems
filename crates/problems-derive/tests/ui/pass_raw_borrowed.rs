use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure<'a> {
    #[error("invalid value")]
    #[problem(
        type_uri = "urn:test:invalid",
        title = "Invalid",
        detail = "{type} / {type:?}"
    )]
    Invalid { r#type: &'a str },
}
fn main() {
    let value = "hello";
    let expected = format!("{type} / {type:?}", r#type = value);
    let failure = Failure::Invalid { r#type: value };
    assert_eq!(failure.detail(), Some(expected));
}
