#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("private")]
#[problem(type_uri = "urn:test")]
struct Failure<'a> {
    #[problem(data)]
    value: &'a str,
}
fn main() {
    let value = String::from("x");
    let report = issues::Report::new(Failure { value: &value });
    let _ = report.as_details();
    let _ = report.into_details();
}
