use issues::StatusCode;
trait Code {
    const STATUS: StatusCode;
}
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Failure<T: std::fmt::Debug + Code> {
    #[error("invalid")]
    #[problem(type_uri = "urn:audit:generic", status = T::STATUS, title = "Generic")]
    Invalid { value: T },
}
#[derive(Debug)]
struct Client;
impl Code for Client {
    const STATUS: StatusCode = StatusCode::BAD_REQUEST;
}
fn main() {}
