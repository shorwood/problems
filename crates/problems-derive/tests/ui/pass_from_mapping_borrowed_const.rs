#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test:mapping")]
#[problem(from(&'a str, |error| Self::Value(error)))]
#[problem(from([u8; N], |_error| Self::Value("array")))]
enum Failure<'a, const N: usize> {
    #[error("invalid value: {0}")]
    Value(&'a str),
}
fn main() {
    let input = String::from("borrowed");
    let failure: Failure<'_, 3> = input.as_str().into();
    assert!(matches!(failure, Failure::Value(value) if value == input));
    let failure: Failure<'_, 3> = [1, 2, 3].into();
    assert!(matches!(failure, Failure::Value("array")));
}
