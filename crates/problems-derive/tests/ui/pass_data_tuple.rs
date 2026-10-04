#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("private")]
#[problem(type_uri = "urn:test")]
struct Failure(#[problem(data = "value")] u8);
fn main() { let details = issues::Report::new(Failure(7)).into_details(); assert_eq!(serde_json::to_value(details).unwrap()["data"]["value"], 7); }
