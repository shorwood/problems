use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Inner<'a> {
    #[error("private {name}")]
    #[problem(detail = "{name}")]
    Borrowed { name: &'a str },
}
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Outer<'a> {
    #[error("{0}")]
    #[problem(transparent)]
    Inner(Inner<'a>),
}
fn main() {
    let name = String::from("borrowed");
    let outer = Outer::Inner(Inner::Borrowed { name: &name });
    assert_eq!(outer.detail().as_deref(), Some("borrowed"));
    assert_eq!(Outer::definitions().count(), 1);
}
