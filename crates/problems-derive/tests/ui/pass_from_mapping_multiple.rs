use issues::{IntoReport, Problem, StatusCode};
use std::error::Error;

#[derive(Debug)]
enum PaginationError {
    InvalidCursor,
    InvalidModel,
    Database(std::io::Error),
}

#[derive(Debug)]
enum ValidationError {
    InvalidCursor,
    InvalidConfiguration,
}

#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test:mapping")]
#[problem(from(PaginationError, |error| match error {
    PaginationError::InvalidCursor => Self::InvalidCursor,
    PaginationError::InvalidModel => Self::InvalidPaginationModel,
    PaginationError::Database(source) => source.into(),
}))]
#[problem(from(ValidationError, |error| match error {
    ValidationError::InvalidCursor => Self::InvalidCursor,
    ValidationError::InvalidConfiguration => Self::InvalidConfiguration,
}))]
enum Failure {
    #[error("invalid cursor")]
    #[problem(400)]
    InvalidCursor,
    #[error("unsupported pagination model")]
    InvalidPaginationModel,
    #[error("invalid configuration")]
    #[problem(422)]
    InvalidConfiguration,
    #[error("storage operation failed")]
    Storage {
        #[from]
        source: std::io::Error,
    },
}

fn propagate<E>(error: E) -> Result<(), Failure>
where
    Failure: From<E>,
{
    Err(error)?
}

fn main() {
    // --- Classify both source enums, including their shared destination.
    assert!(matches!(
        Failure::from(PaginationError::InvalidCursor),
        Failure::InvalidCursor
    ));
    assert!(matches!(
        Failure::from(PaginationError::InvalidModel),
        Failure::InvalidPaginationModel
    ));
    let failure: Failure = ValidationError::InvalidCursor.into();
    assert!(matches!(failure, Failure::InvalidCursor));
    assert!(matches!(
        Failure::from(ValidationError::InvalidConfiguration),
        Failure::InvalidConfiguration
    ));
    assert!(matches!(
        propagate(PaginationError::InvalidCursor),
        Err(Failure::InvalidCursor)
    ));
    assert!(matches!(
        propagate(ValidationError::InvalidConfiguration),
        Err(Failure::InvalidConfiguration)
    ));

    // --- Retain the forwarded database cause without exposing it in the document.
    let source = std::io::Error::other("SECRET database cause");
    let failure: Failure = PaginationError::Database(source).into();
    assert_eq!(
        failure.source().unwrap().to_string(),
        "SECRET database cause"
    );
    assert_eq!(failure.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = serde_json::to_value(failure.into_report().as_details()).unwrap();
    assert!(body.get("detail").is_none());
    assert!(!body.to_string().contains("SECRET"));
}
