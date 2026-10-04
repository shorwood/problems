use problems::{Problem, ProblemDefinition, ProblemDocument, Report, StatusCode};

#[test]
fn received_document_owns_all_members() {
    let document = {
        let buffer = String::from(
            r#"{"type":"urn:test:conflict","title":"Conflict","status":409,"detail":"Choose another name.","instance":"/occurrences/123"}"#,
        );
        serde_json::from_str::<ProblemDocument>(&buffer).unwrap()
    };
    assert_eq!(document.type_uri(), "urn:test:conflict");
    assert_eq!(document.title(), Some("Conflict"));
    assert_eq!(document.status(), Some(StatusCode::CONFLICT));
    assert_eq!(document.detail(), Some("Choose another name."));
    assert_eq!(document.instance(), Some("/occurrences/123"));
}

#[test]
fn omitted_members_have_receiving_defaults() {
    let document: ProblemDocument = serde_json::from_str("{}").unwrap();
    assert_eq!(document.type_uri(), "about:blank");
    assert_eq!(document.title(), None);
    assert_eq!(document.status(), None);
    assert_eq!(document.detail(), None);
    assert_eq!(document.instance(), None);
    assert_eq!(document.data(), None);
    for field in ["type", "title", "status", "detail", "instance"] {
        let mut value = serde_json::json!({
            "type": "urn:test:conflict", "title": "Conflict", "status": 409,
            "detail": "Public explanation", "instance": "urn:test:occurrence"
        });
        value.as_object_mut().unwrap().remove(field);
        let document: ProblemDocument = serde_json::from_value(value).unwrap();
        match field {
            "type" => assert_eq!(document.type_uri(), "about:blank"),
            "title" => assert_eq!(document.title(), None),
            "status" => assert_eq!(document.status(), None),
            "detail" => assert_eq!(document.detail(), None),
            "instance" => assert_eq!(document.instance(), None),
            _ => unreachable!(),
        }
    }
}

#[test]
fn received_data_preserves_arbitrary_json() {
    for data in [
        serde_json::json!({"nested": [1, {"enabled": true, "optional": null}]}),
        serde_json::json!([1, "two", false]),
        serde_json::json!("message"),
        serde_json::json!(42),
        serde_json::json!(true),
    ] {
        let document: ProblemDocument<serde_json::Value> =
            serde_json::from_value(serde_json::json!({"data": data})).unwrap();
        assert_eq!(document.data(), Some(&data));
    }
    let document: ProblemDocument = serde_json::from_str(r#"{"data":null}"#).unwrap();
    assert_eq!(document.data(), None);
}

#[test]
fn wrongly_typed_members_fail_decoding() {
    for input in [
        r#"{"type":123}"#,
        r#"{"title":true}"#,
        r#"{"status":"409"}"#,
        r#"{"detail":[]}"#,
        r#"{"instance":{}}"#,
    ] {
        assert!(serde_json::from_str::<ProblemDocument>(input).is_err());
    }
}

#[test]
fn unknown_extensions_do_not_hide_standard_members() {
    let document: ProblemDocument = serde_json::from_str(
        r#"{"extension":{"nested":[1,{"x":true}]},"status":499,"title":"Custom"}"#,
    )
    .unwrap();
    assert_eq!(document.status(), Some(StatusCode::from_u16(499).unwrap()));
    assert_eq!(document.title(), Some("Custom"));
}

#[test]
fn unsupported_status_fails_decoding() {
    for input in [r#"{"status":99}"#, r#"{"status":1000}"#] {
        assert!(serde_json::from_str::<ProblemDocument>(input).is_err());
    }
}

#[derive(Debug, thiserror::Error)]
#[error("private diagnostic")]
struct Failure(Option<String>);

impl Problem for Failure {
    type Data = ();

    type DataRef<'data>
        = ()
    where
        Self: 'data;
    fn definition(&self) -> &'static ProblemDefinition {
        &ProblemDefinition {
            type_uri: "urn:test:failure",
            title: "Failure",
            status: StatusCode::CONFLICT,
        }
    }

    fn detail(&self) -> Option<String> {
        self.0.clone()
    }

    fn definitions() -> impl Iterator<Item = &'static ProblemDefinition> {
        std::iter::once(Failure(None).definition())
    }
}

#[test]
fn producer_conversion_preserves_metadata_and_optional_omission() {
    let document = ProblemDocument::from(Report::new(Failure(None)).into_details());
    assert_eq!(document.type_uri(), "urn:test:failure");
    assert_eq!(document.title(), Some("Failure"));
    assert_eq!(document.status(), Some(StatusCode::CONFLICT));
    assert_eq!(document.detail(), None);
    assert_eq!(document.instance(), None);
}

#[test]
fn producer_conversion_moves_report_instance() {
    let instance = String::from("urn:test:occurrence");
    let allocation = instance.as_ptr();
    let document = ProblemDocument::from(
        Report::new(Failure(None))
            .with_instance(instance)
            .into_details(),
    );
    assert_eq!(document.instance(), Some("urn:test:occurrence"));
    assert_eq!(document.instance().unwrap().as_ptr(), allocation);
}

#[test]
fn producer_conversion_moves_present_detail() {
    let details = Report::new(Failure(Some("Public explanation".into()))).into_details();
    let allocation = details.detail().unwrap().as_ptr();
    let document = ProblemDocument::from(details);
    assert_eq!(document.detail(), Some("Public explanation"));
    assert_eq!(document.detail().unwrap().as_ptr(), allocation);
}

#[cfg(feature = "derive")]
#[test]
fn matches_variant_definition_without_constructing_error() {
    #[derive(Debug, thiserror::Error, problems::Problem)]
    #[problem(prefix = "urn:test")]
    #[allow(dead_code)]
    enum Local {
        #[error("private: {source}")]
        #[problem(409)]
        NameConflict { source: std::io::Error },
        #[error("private")]
        #[problem(404)]
        Missing,
    }
    let document: ProblemDocument = serde_json::from_str(
        r#"{"type":"urn:test:name-conflict","title":"Other title","status":500,"instance":"urn:test:occurrence"}"#,
    ).unwrap();
    assert!(document.is_type(&Local::NAME_CONFLICT));
    assert!(!document.is_type(&Local::MISSING));
}

#[cfg(feature = "schemars")]
#[test]
fn receiving_schema_preserves_defaults_and_optional_members() {
    let schema = serde_json::to_value(schemars::schema_for!(ProblemDocument)).unwrap();
    let properties = schema["properties"].as_object().unwrap();
    assert_eq!(properties.len(), 6);
    assert_eq!(properties["type"]["type"], "string");
    assert_eq!(properties["type"]["default"], "about:blank");
    assert!(
        schema
            .get("required")
            .is_none_or(|required| required.as_array().unwrap().is_empty())
    );
    let status = &properties["status"];
    assert_eq!(status["type"], serde_json::json!(["integer", "null"]));
    assert_eq!(status["minimum"], 100);
    assert_eq!(status["maximum"], 999);
    assert_ne!(schema["additionalProperties"], false);
}
