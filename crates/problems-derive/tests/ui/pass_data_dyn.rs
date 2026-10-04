#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("private")]
#[problem(type_uri = "urn:test")]
struct Failure;
fn main() {
    let problem: &dyn issues::Problem = &Failure;
    let _ = problem.definition();
}
