use issues::IntoReport;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]

enum Failure {
    #[error("failure")]
    #[problem(type_uri = "https://example.com/Custom_URI")]
    NameConflict,
}
fn main() {
    let body = serde_json::to_value(Failure::NameConflict.into_report().as_details()).unwrap();
    assert_eq!(body["type"], "https://example.com/Custom_URI");
}
