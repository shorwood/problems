use issues::IntoReport;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure<T> {
    #[error("private diagnostic")]
    #[problem(409)]
    Conflict(T, u32),
}
fn main() {
    let report = Failure::Conflict(std::io::Error::other("private source"), 42).into_report();
    assert_eq!(report.as_details().status(), issues::StatusCode::CONFLICT);
    assert!(report.as_details().detail().is_none());
}
