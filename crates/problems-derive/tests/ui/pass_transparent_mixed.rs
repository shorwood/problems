use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Inner {
    #[error("private {name}")]
    #[problem(status = 409, detail = "Name: {name}")]
    Conflict { name: String },
}
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:outer")]
enum Outer {
    #[error("before")]
    #[problem(400)]
    Before,
    #[error(transparent)]
    #[problem(transparent)]
    Inner(#[from] Inner),
    #[error("after")]
    #[problem(503)]
    After,
}
fn main() {
    assert_eq!(
        Outer::definitions()
            .map(|d| d.status.as_u16())
            .collect::<Vec<_>>(),
        [400, 409, 503]
    );
    assert_eq!(Outer::Before.definition().type_uri, "urn:outer:before");
    assert_eq!(Outer::After.definition().type_uri, "urn:outer:after");
    assert_eq!(
        Outer::from(Inner::Conflict {
            name: "example".into()
        })
        .definition()
        .type_uri,
        "urn:test:conflict"
    );
}
