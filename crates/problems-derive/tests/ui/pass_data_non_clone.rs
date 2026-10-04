#[derive(Debug, issues::__private::serde::Serialize)]
#[serde(crate = "issues::__private::serde")]
struct Value(u8);
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("private")]
#[problem(type_uri = "urn:test")]
struct Failure { #[problem(data)] value: Value }
fn main() { let report = issues::Report::new(Failure { value: Value(1) }); let _ = serde_json::to_value(report.as_details()); let _ = report.into_details(); }
