use issues::{Problem, ProblemDefinition, StatusCode};
#[derive(Debug, thiserror::Error)]
#[error("private")]
struct Inner;
impl Problem for Inner {
    fn definition(&self) -> &'static ProblemDefinition {
        &ProblemDefinition {
            type_uri: "urn:test:inner",
            title: "Inner",
            status: StatusCode::BAD_REQUEST,
        }
    }
    fn definitions() -> impl Iterator<Item = &'static ProblemDefinition> {
        std::iter::once(Inner.definition())
    }
    fn instance(&self) -> Option<String> {
        Some("/occurrences/123".into())
    }
}
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Outer {
    #[error(transparent)]
    #[problem(transparent)]
    Inner(#[from] Inner),
}
fn main() {
    let outer = Outer::from(Inner);
    let problem: &dyn Problem = &outer;
    assert_eq!(problem.instance().as_deref(), Some("/occurrences/123"));
}
