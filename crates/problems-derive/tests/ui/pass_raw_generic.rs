use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure<T> {
    #[error("invalid value")]
    #[problem(
        type_uri = "urn:test:invalid",
        title = "Invalid",
        detail = "{type} / {type:?}"
    )]
    Invalid { r#type: T },
}
fn main() {
    let value = 42u32;
    let expected = format!("{type} / {type:?}", r#type = value);
    let failure = Failure::Invalid { r#type: value };
    assert_eq!(failure.detail(), Some(expected));
}
