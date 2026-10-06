struct GenericSource<T>(T);

#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test:generic-mapping", from(GenericSource<T>, |error| Self::Value(error.0)))]
enum GenericFailure<T>
where
    T: std::fmt::Debug + std::fmt::Display,
{
    #[error("invalid value: {0}")]
    Value(T),
}

fn main() {
    let failure: GenericFailure<String> = GenericSource("value".to_owned()).into();
    assert!(matches!(failure, GenericFailure::Value(value) if value == "value"));
}
