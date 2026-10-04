use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("unavailable")]
#[problem(type_uri = "urn:test:unavailable")]
struct ServiceUnavailable;
fn main() {
    assert_eq!(ServiceUnavailable::DEFINITION.title, "Service Unavailable");
    assert_eq!(ServiceUnavailable::DEFINITION.status, 500);
    assert_eq!(
        ServiceUnavailable.definition(),
        &ServiceUnavailable::DEFINITION
    );
    assert_eq!(
        ServiceUnavailable::definitions().collect::<Vec<_>>(),
        vec![&ServiceUnavailable::DEFINITION]
    );
    assert_eq!(ServiceUnavailable.detail(), None);
}
