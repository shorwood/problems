use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("invalid")]
    #[problem(
        type_uri = "urn:test:invalid",
        title = "Invalid",
        detail = "{value:08x}"
    )]
    Invalid { value: u32 },
}
fn main() {
    let value = 42;
    let expected = format!("{value:08x}", value = value);
    assert_eq!(Failure::Invalid { value }.detail(), Some(expected));
}
