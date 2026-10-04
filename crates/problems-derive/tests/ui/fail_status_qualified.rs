trait Code {
    const STATUS: issues::StatusCode;
}
struct Client;
impl Code for Client {
    const STATUS: issues::StatusCode = issues::StatusCode::CONFLICT;
}
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure {
    #[error("failure")]
    #[problem(type_uri = "urn:test:failure", title = "Failure", status = <Client as Code>::STATUS)]
    Failed,
}
fn main() {}
