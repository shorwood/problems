use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("invalid input")]
#[problem(type_uri = "urn:test:invalid", detail = "{value:04x} / {label}")]
struct InvalidInput<'a, T> { value: T, label: &'a str }
fn main() {
    let label = String::from("value");
    assert_eq!(InvalidInput { value: 15u32, label: &label }.detail().as_deref(), Some("000f / value"));
}
