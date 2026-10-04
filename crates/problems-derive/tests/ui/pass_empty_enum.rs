use issues::Problem;
#[derive(Debug, thiserror::Error, problems_derive::Problem)]
enum Never {}
fn main() {
    assert!(Never::definitions().next().is_none());
}
