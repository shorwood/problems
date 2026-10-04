use issues::{Problem, Report};

#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[error("retry later")]
#[problem(type_uri = "urn:test:retry")]
#[problem(0x199)]
struct Retry {
    #[problem(data)]
    seconds: u16,
}

#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error(transparent)]
    #[problem(transparent)]
    Retry(Retry),
    #[error("missing {resource}")]
    #[problem(status = 404)]
    #[problem(detail = "Missing: {resource}")]
    Missing {
        #[problem(data = "name")]
        resource: String,
    },
}

fn main() {
    let retry = Report::new(Failure::Retry(Retry { seconds: 30 }));
    let borrowed = serde_json::to_value(retry.as_details()).unwrap();
    let owned = serde_json::to_value(retry.into_details()).unwrap();
    assert_eq!(borrowed, owned);
    assert_eq!(owned["status"], 409);
    assert_eq!(owned["data"]["seconds"], 30);

    let missing = Failure::Missing {
        resource: "record".into(),
    };
    assert_eq!(missing.definition().type_uri, "urn:test:missing");
    assert_eq!(missing.detail().as_deref(), Some("Missing: record"));
    let owned = serde_json::to_value(Report::new(missing).into_details()).unwrap();
    assert_eq!(owned["data"]["name"], "record");
    assert_eq!(Failure::definitions().count(), 2);
}
