use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("internal: {cause}")]
    #[problem(type_uri = "urn:test:internal", title = "Internal error")]
    Internal {
        #[from]
        cause: std::io::Error,
    },
}
fn main() {
    let failure = Failure::from(std::io::Error::other("password=SECRET"));
    assert_eq!(
        std::error::Error::source(&failure).unwrap().to_string(),
        "password=SECRET"
    );
    assert_eq!(
        failure.definition().status,
        issues::StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(failure.detail(), None);
}
