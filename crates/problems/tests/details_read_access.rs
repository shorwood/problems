use problems::{Problem, ProblemDefinition, Report, StatusCode};

#[derive(Debug, thiserror::Error)]
#[error("private diagnostic")]
struct Failure {
    detail: Option<String>,
}

static DEFINITION: ProblemDefinition = ProblemDefinition {
    type_uri: "urn:test:failure",
    title: "Failure",
    status: StatusCode::CONFLICT,
};

impl Problem for Failure {
    fn definition(&self) -> &'static ProblemDefinition {
        &DEFINITION
    }

    fn definitions() -> impl Iterator<Item = &'static ProblemDefinition> {
        std::iter::once(&DEFINITION)
    }

    fn detail(&self) -> Option<String> {
        self.detail.clone()
    }
}

#[test]
fn optional_getters_agree_with_serialization() {
    for present in [false, true] {
        let mut report = Report::new(Failure {
            detail: present.then(|| "Public explanation".into()),
        });
        if present {
            report = report.with_instance("urn:test:occurrence");
        }
        let document = report.into_details();
        let body = serde_json::to_value(&document).unwrap();
        assert_eq!(document.detail(), present.then_some("Public explanation"));
        assert_eq!(
            document.instance(),
            present.then_some("urn:test:occurrence")
        );
        assert_eq!(
            document.detail(),
            body.get("detail").and_then(|v| v.as_str())
        );
        assert_eq!(
            document.instance(),
            body.get("instance").and_then(|v| v.as_str())
        );
    }
}
