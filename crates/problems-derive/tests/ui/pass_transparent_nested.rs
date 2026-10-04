use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Inner {
    #[error("private {name}")]
    #[problem(status = 409, detail = "Name: {name}")]
    Conflict { name: String },
}
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Middle {
    #[error(transparent)]
    #[problem(transparent)]
    Inner(#[from] Inner),
}
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Outer {
    #[error(transparent)]
    #[problem(transparent)]
    Middle(#[from] Middle),
}
fn main() {
    let outer = Outer::from(Middle::from(Inner::Conflict {
        name: "example".into(),
    }));
    assert_eq!(outer.detail().as_deref(), Some("Name: example"));
    assert_eq!(Outer::definitions().count(), 1);
}
