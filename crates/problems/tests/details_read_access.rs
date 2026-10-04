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

#[test]
fn typed_status_preserves_custom_numeric_projection() {
    #[derive(Debug, thiserror::Error)]
    #[error("private diagnostic")]
    struct Custom;

    static CUSTOM: ProblemDefinition = ProblemDefinition {
        type_uri: "urn:test:custom",
        title: "Custom",
        status: match StatusCode::from_u16(499) {
            Ok(status) => status,
            Err(_) => panic!("invalid fixture status"),
        },
    };

    impl Problem for Custom {
        fn definition(&self) -> &'static ProblemDefinition {
            &CUSTOM
        }

        fn definitions() -> impl Iterator<Item = &'static ProblemDefinition> {
            std::iter::once(&CUSTOM)
        }
    }

    let document = Report::new(Custom).into_details();
    assert_eq!(document.status(), CUSTOM.status);
    assert!(document.status().is_client_error());
    assert_eq!(serde_json::to_value(&document).unwrap()["status"], 499);
    assert_eq!(
        problems::GenericProblem::from(document).status(),
        Some(CUSTOM.status)
    );
}

#[cfg(feature = "schemars")]
#[test]
fn typed_status_keeps_numeric_required_schema() {
    let schema = serde_json::to_value(schemars::schema_for!(problems::ProblemDetails)).unwrap();
    let status = &schema["properties"]["status"];
    assert_eq!(status["type"], "integer");
    assert_eq!(status["minimum"], 100);
    assert_eq!(status["maximum"], 999);
    assert!(status["description"].as_str().unwrap().contains("advisory"));
    for code in [100, 400, 499, 500, 999] {
        assert!(StatusCode::from_u16(code).is_ok());
        assert!(
            (status["minimum"].as_u64().unwrap()..=status["maximum"].as_u64().unwrap())
                .contains(&u64::from(code))
        );
    }
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "status")
    );
}
