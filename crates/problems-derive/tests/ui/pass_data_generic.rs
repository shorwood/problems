#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("private")]
#[problem(type_uri = "urn:test")]
struct Failure<T: std::fmt::Debug> {
    #[problem(data)]
    value: T,
}
fn main() {
    let report = issues::Report::new(Failure { value: 7u8 });
    let _ = report.as_details();
}
