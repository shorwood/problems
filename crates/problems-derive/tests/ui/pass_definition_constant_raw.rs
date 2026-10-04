#![allow(non_camel_case_types)]
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("failure")]
    r#type,
}
fn main() {
    assert_eq!(Failure::TYPE.type_uri, "urn:test:type");
}
