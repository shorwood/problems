use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("invalid")]
    #[problem(
        type_uri = "urn:test:invalid",
        title = "Invalid",
        detail = "{value:$>8}"
    )]
    Invalid { value: String },
}
fn main() {
    let value = "hello".to_owned();
    let expected = format!("{value:$>8}", value = value);
    assert_eq!(Failure::Invalid { value }.detail(), Some(expected));
}
