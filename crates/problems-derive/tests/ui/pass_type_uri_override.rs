use issues::IntoReport;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("failure")]
    #[problem(type_uri = "urn:custom:conflict")]
    NameConflict,
    #[error("failure")]
    Unavailable,
}
fn main() {
    let overridden =
        serde_json::to_value(Failure::NameConflict.into_report().as_details()).unwrap();
    let inherited = serde_json::to_value(Failure::Unavailable.into_report().as_details()).unwrap();
    assert_eq!(overridden["type"], "urn:custom:conflict");
    assert_eq!(inherited["type"], "urn:test:unavailable");
}
