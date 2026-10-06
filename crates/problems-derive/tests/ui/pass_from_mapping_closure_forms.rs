#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(
    from((u8, u16), |(small_value, large_value)| { Self::Value(u32::from(small_value) + u32::from(large_value)) },),
    prefix = "urn:test:mapping",
    from(bool, move |flag: bool| -> Self { Self::Value(u32::from(flag)) }),
)]
enum Failure {
    #[error("invalid value: {0}")]
    Value(u32),
}
fn main() {
    let failure: Failure = (2_u8, 3_u16).into();
    assert!(matches!(failure, Failure::Value(5)));
    let failure: Failure = true.into();
    assert!(matches!(failure, Failure::Value(1)));
}
