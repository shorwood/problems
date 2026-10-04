use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Conditional {
    #[cfg(any())]
    #[error("disabled")]
    #[problem(type_uri = "urn:test:disabled", title = "Disabled")]
    Removed,
    #[error("first")]
    #[problem(type_uri = "urn:test:first", title = "First", status = 404)]
    First,
    #[cfg(any())]
    #[error("also disabled")]
    #[problem(type_uri = "urn:test:also-disabled", title = "Disabled")]
    AlsoRemoved,
    #[error("second {value}")]
    #[problem(
        type_uri = "urn:test:second",
        title = "Second",
        detail = "Value: {value}"
    )]
    Second { value: String },
}
fn main() {
    let definitions = Conditional::definitions().collect::<Vec<_>>();
    assert_eq!(definitions.len(), 2);
    assert!(std::ptr::eq(
        Conditional::First.definition(),
        definitions[0]
    ));
    assert_eq!(definitions[0].status, issues::StatusCode::NOT_FOUND);
    assert_eq!(Conditional::First.detail(), None);
    let second = Conditional::Second {
        value: "public".into(),
    };
    assert!(std::ptr::eq(second.definition(), definitions[1]));
    assert_eq!(definitions[1].type_uri, "urn:test:second");
    assert_eq!(
        definitions[1].status,
        issues::StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(second.detail().as_deref(), Some("Value: public"));
}
