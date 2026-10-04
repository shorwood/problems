use problems::{Problem, ProblemDefinition, ProblemDetails, ProblemDocument, Report, StatusCode};

#[derive(Debug, thiserror::Error)]
#[error("private diagnostic")]
struct Failure(bool);

static DEFINITION: ProblemDefinition = ProblemDefinition {
    type_uri: "urn:test:conflict",
    title: "Conflict",
    status: StatusCode::CONFLICT,
};

impl Problem for Failure {
    type Data = ();

    type DataRef<'data>
        = ()
    where
        Self: 'data;
    fn definition(&self) -> &'static ProblemDefinition {
        &DEFINITION
    }

    fn definitions() -> impl Iterator<Item = &'static ProblemDefinition> {
        std::iter::once(&DEFINITION)
    }

    fn detail(&self) -> Option<String> {
        self.0.then(|| "Choose another name.".into())
    }
}

fn details() -> ProblemDetails {
    Report::new(Failure(true))
        .with_instance("urn:test:occurrence:1")
        .into_details()
}

#[test]
fn converted_and_decoded_documents_are_equal() {
    fn requires_eq<T: Eq>(_: &T) {}
    let producer = details();
    let received = ProblemDocument::from(producer.clone());
    requires_eq(&producer);
    requires_eq(&received);
    assert_eq!(producer, producer.clone());
    assert_eq!(received, received.clone());
    let decoded: ProblemDocument =
        serde_json::from_value(serde_json::to_value(&producer).unwrap()).unwrap();
    assert_eq!(received, decoded);
    assert_eq!(decoded, received);
}

#[test]
fn instance_differences_match_failure_but_not_document() {
    let producer = details();
    let expected = ProblemDocument::from(producer.clone());
    for instance in [None, Some("urn:test:occurrence:2")] {
        let mut body = serde_json::to_value(&producer).unwrap();
        body.as_object_mut().unwrap().remove("instance");
        if let Some(instance) = instance {
            body["instance"] = instance.into();
        }
        let received: ProblemDocument = serde_json::from_value(body).unwrap();
        assert!(received.matches(&producer));
        assert!(received.is_type(&DEFINITION));
        assert_ne!(received, expected);
        assert_ne!(expected, received);
    }
    let other_occurrence = Report::new(Failure(true))
        .with_instance("urn:test:occurrence:2")
        .into_details();
    assert_ne!(producer, other_occurrence);
}

#[test]
fn matching_requires_every_failure_member() {
    let producer = details();
    let expected = ProblemDocument::from(producer.clone());
    for (field, replacement) in [
        ("type", serde_json::json!("urn:test:other")),
        ("title", serde_json::json!("Other")),
        ("status", serde_json::json!(500)),
        ("detail", serde_json::json!("Different explanation.")),
        ("title", serde_json::Value::Null),
        ("status", serde_json::Value::Null),
        ("detail", serde_json::Value::Null),
        ("detail", serde_json::json!("")),
    ] {
        let mut body = serde_json::to_value(&producer).unwrap();
        body[field] = replacement;
        let received: ProblemDocument = serde_json::from_value(body).unwrap();
        assert!(!received.matches(&producer), "{field}");
        assert_ne!(received, expected);
        assert_ne!(expected, received);
    }
}

#[test]
fn omitted_optional_members_match_each_other() {
    let producer = Report::new(Failure(false)).into_details();
    let received: ProblemDocument =
        serde_json::from_value(serde_json::to_value(&producer).unwrap()).unwrap();
    assert_eq!(received.instance(), None);
    assert_eq!(received.detail(), None);
    assert!(received.matches(&producer));
    let expected = ProblemDocument::from(producer);
    assert_eq!(received, expected);
}
