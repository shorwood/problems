#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("invalid")]
#[problem(type_uri = "urn:test:struct", from(bool, |_error| Self))]
struct InvalidStruct;

#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test:variant")]
enum InvalidVariant {
    #[error("invalid")]
    #[problem(from(bool, |_error| Self::Invalid))]
    Invalid,
}

#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test:field")]
enum InvalidField {
    #[error("invalid")]
    Invalid {
        #[problem(from(bool, |_error| Self::Invalid { value: false }))]
        value: bool,
    },
}
fn main() {}
