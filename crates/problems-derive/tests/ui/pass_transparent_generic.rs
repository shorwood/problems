use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Inner {
    #[error("private {name}")]
    #[problem(status = 409, detail = "Name: {name}")]
    Conflict { name: String },
}
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Outer<T> {
    #[error(transparent)]
    #[problem(transparent)]
    Inner(#[from] T),
}
fn main() {
    let outer = Outer::from(Inner::Conflict {
        name: "example".into(),
    });
    assert_eq!(outer.definition().status, 409);
    assert_eq!(Outer::<Inner>::definitions().count(), 1);
}
