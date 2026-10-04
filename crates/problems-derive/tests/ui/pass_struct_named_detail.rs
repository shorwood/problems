use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("name taken")]
#[problem(
    type_uri = "urn:test:name-conflict",
    status = 409,
    detail = "Name {name} is taken."
)]
struct NameConflict {
    name: String,
}
fn main() {
    assert_eq!(
        NameConflict {
            name: "alice".into()
        }
        .detail()
        .as_deref(),
        Some("Name alice is taken.")
    );
}
