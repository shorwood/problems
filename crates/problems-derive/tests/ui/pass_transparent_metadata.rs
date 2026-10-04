use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Inner {
    #[error("private {name}")]
    #[problem(status = 409, detail = "Name: {name}")]
    Conflict { name: String },
}
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Outer {
    #[error(transparent)]
    #[problem(transparent)]
    Inner(#[from] Inner),
}
fn main() {
    let outer = Outer::from(Inner::Conflict {
        name: "example".into(),
    });
    assert_eq!(outer.definition().type_uri, "urn:test:conflict");
    assert_eq!(outer.definition().status, 409);
    assert_eq!(
        Outer::definitions().map(|d| d.type_uri).collect::<Vec<_>>(),
        ["urn:test:conflict"]
    );
}
