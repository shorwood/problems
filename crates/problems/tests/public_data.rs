#![cfg(feature = "derive")]

use problems::{IntoReport, ProblemDocument};

#[derive(Debug, thiserror::Error, problems::Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("retry")]
    #[problem(status = 429, detail = "Try again in {seconds} seconds.")]
    Retry {
        #[problem(data)]
        seconds: u32,
        #[problem(data = "request")]
        request_id: String,
        source: std::io::Error,
    },
    #[error("missing")]
    #[problem(404)]
    Missing,
}

#[test]
fn selected_fields_round_trip_and_compare() {
    let report = Failure::Retry {
        seconds: 30,
        request_id: "abc".into(),
        source: std::io::Error::other("private"),
    }
    .into_report()
    .with_instance("/occurrences/1");
    let expected = serde_json::json!({
        "type": "urn:test:retry", "title": "Retry", "status": 429,
        "detail": "Try again in 30 seconds.", "instance": "/occurrences/1",
        "data": {"seconds": 30, "request": "abc"}
    });
    assert_eq!(serde_json::to_value(report.as_details()).unwrap(), expected);
    let received: ProblemDocument<FailureData<u32, String>> =
        serde_json::from_value(expected.clone()).unwrap();
    let moved = report.into_details();
    assert!(received.matches(&moved));
    assert_eq!(received.data(), moved.data());
    assert_eq!(ProblemDocument::from(moved), received);
}

#[derive(Debug, thiserror::Error, problems::Problem)]
#[error("retry after {seconds} seconds")]
#[problem(
    type_uri = "urn:test:typed-retry",
    title = "Retry",
    status = 429,
    detail = "Try again later."
)]
struct TypedRetry {
    #[problem(data)]
    seconds: u32,
    #[problem(data)]
    request: String,
}

#[test]
fn json_and_xml_match_typed_producer_details() {
    let expected = TypedRetry {
        seconds: 30,
        request: "abc".into(),
    }
    .into_report()
    .with_instance("/occurrences/expected")
    .into_details();

    type Received = ProblemDocument<TypedRetryData<u32, String>>;
    let from_json: Received = serde_json::from_str(
        r#"{
            "type": "urn:test:typed-retry",
            "title": "Retry",
            "status": 429,
            "detail": "Try again later.",
            "instance": "/occurrences/json",
            "data": {"seconds": 30, "request": "abc"}
        }"#,
    )
    .unwrap();
    let from_xml: Received = serde_xml_rs::from_str(
        r#"<problem xmlns="urn:ietf:rfc:7807">
            <type>urn:test:typed-retry</type>
            <title>Retry</title>
            <status>429</status>
            <detail>Try again later.</detail>
            <instance>/occurrences/xml</instance>
            <data><seconds>30</seconds><request>abc</request></data>
        </problem>"#,
    )
    .unwrap();

    assert!(from_json.matches(&expected));
    assert!(from_xml.matches(&expected));
    assert_eq!(from_json.data(), from_xml.data());
    assert_ne!(from_json, from_xml);

    for data in [
        serde_json::Value::Null,
        serde_json::json!({"seconds": 31, "request": "abc"}),
    ] {
        let mut body = serde_json::to_value(&expected).unwrap();
        body["data"] = data;
        let received: Received = serde_json::from_value(body).unwrap();
        assert!(!received.matches(&expected));
    }
    let mut body = serde_json::to_value(&expected).unwrap();
    body.as_object_mut().unwrap().remove("data");
    let received: Received = serde_json::from_value(body).unwrap();
    assert_eq!(received.data(), None);
    assert!(!received.matches(&expected));
}

#[test]
fn no_selected_fields_omit_data() {
    assert!(
        serde_json::to_value(Failure::Missing.into_report().into_details())
            .unwrap()
            .get("data")
            .is_none()
    );
}

#[derive(Debug, problems::__private::serde::Serialize)]
struct NonClone(u32);

#[derive(Debug, thiserror::Error, problems::Problem)]
#[error("private")]
#[problem(type_uri = "urn:test:owned")]
struct Owned {
    #[problem(data)]
    value: NonClone,
}

#[test]
fn non_clone_public_fields_can_be_borrowed_and_moved() {
    let report = Owned { value: NonClone(7) }.into_report();
    assert_eq!(
        serde_json::to_value(report.as_details()).unwrap()["data"]["value"],
        7
    );
    let document = ProblemDocument::from(report.into_details());
    assert_eq!(document.data().unwrap().value.0, 7);
}

#[derive(Debug, thiserror::Error, problems::Problem)]
#[problem(prefix = "urn:test:handler")]
enum Handler {
    #[error(transparent)]
    #[problem(transparent)]
    Inner(#[from] Failure),
    #[error("tuple")]
    #[problem(409)]
    Tuple(#[problem(data = "count")] usize),
}

#[test]
fn transparent_composition_preserves_borrowed_and_moved_wire_shapes() {
    let report = Handler::Tuple(3).into_report();
    assert_eq!(
        serde_json::to_value(report.as_details()).unwrap()["data"],
        serde_json::json!({"count":3})
    );
    let report = Handler::Inner(Failure::Retry {
        seconds: 2,
        request_id: "def".into(),
        source: std::io::Error::other("private"),
    })
    .into_report();
    assert_eq!(
        serde_json::to_value(report.as_details()).unwrap()["data"],
        serde_json::json!({"seconds":2,"request":"def"})
    );
    assert_eq!(
        serde_json::to_value(report.into_details()).unwrap()["data"]["seconds"],
        2
    );
}

#[cfg(feature = "schemars")]
#[test]
fn schema_contains_typed_public_payloads() {
    let schema = schemars::schema_for!(
        problems::ProblemDetails<HandlerData<FailureData<u32, String>, usize>>
    );
    let json = serde_json::to_value(schema).unwrap().to_string();
    assert!(json.contains("seconds"));
    assert!(json.contains("request"));
    assert!(json.contains("count"));
    assert!(json.contains("integer"));

    let schema = schemars::schema_for!(ProblemDocument<OptionalData<Option<u32>>>);
    let json = serde_json::to_value(schema).unwrap().to_string();
    assert!(json.contains("value"));
    assert!(json.contains("integer"));
}

#[derive(Debug, thiserror::Error, problems::Problem)]
#[error("optional")]
#[problem(type_uri = "urn:test:optional")]
struct Optional {
    #[problem(data)]
    value: Option<u32>,
}

#[test]
fn selected_null_is_retained_and_payload_validation_is_explicit() {
    let details = Optional { value: None }.into_report().into_details();
    assert_eq!(
        serde_json::to_value(&details).unwrap()["data"],
        serde_json::json!({"value":null})
    );
    let wrong = serde_json::json!({"data":{"value":"wrong"}});
    let received: ProblemDocument<serde_json::Value> =
        serde_json::from_value(wrong.clone()).unwrap();
    assert_eq!(received.data(), Some(&wrong["data"]));
    assert!(serde_json::from_value::<ProblemDocument<OptionalData<Option<u32>>>>(wrong).is_err());
}

#[test]
fn payload_differences_affect_equality_and_matching() {
    let first = Optional { value: Some(1) }.into_report().into_details();
    let second = Optional { value: Some(2) }.into_report().into_details();
    let received = ProblemDocument::from(first);
    assert!(!received.matches(&second));
    assert_ne!(received, ProblemDocument::from(second));
}

#[derive(Debug)]
struct Broken;

impl serde::Serialize for Broken {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("public serializer failed"))
    }
}

#[derive(Debug, thiserror::Error, problems::Problem)]
#[error("broken")]
#[problem(type_uri = "urn:test:broken", status = 409)]
struct BrokenProblem {
    #[problem(data)]
    value: Broken,
}

#[test]
fn projection_does_not_serialize_public_fields() {
    let report = BrokenProblem { value: Broken }.into_report();
    assert!(report.as_details().data().is_some());
    assert!(serde_json::to_value(report.as_details()).is_err());
    assert!(report.into_details().data().is_some());
}

#[test]
fn document_conversion_moves_payload_without_serialization() {
    let details = BrokenProblem { value: Broken }.into_report().into_details();
    let document = ProblemDocument::from(details);
    assert!(document.data().is_some());
    let error = serde_json::to_value(document.data()).unwrap_err();
    assert!(error.to_string().contains("public serializer failed"));
}

#[cfg(feature = "axum")]
#[tokio::test]
async fn borrowed_axum_encoder_handles_failure_without_consuming_error() {
    use axum::response::IntoResponse;
    let report = BrokenProblem { value: Broken }.into_report();
    let response = (&report).into_response();
    assert_eq!(response.status(), problems::StatusCode::CONFLICT);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(
        std::str::from_utf8(&body)
            .unwrap()
            .contains("public serializer failed")
    );
    assert!(report.as_details().data().is_some());
}

#[cfg(feature = "aide")]
#[test]
fn openapi_shares_typed_payload_union_across_statuses() {
    async fn handler() -> Result<(), problems::Report<Handler>> {
        Err(Handler::Tuple(3).into_report())
    }
    aide::generate::in_context(|context| {
        context.schema = Default::default();
    });
    let mut api = aide::openapi::OpenApi::default();
    let _: axum::Router = aide::axum::ApiRouter::new()
        .api_route("/", aide::axum::routing::get(handler))
        .finish_api(&mut api);
    let json = serde_json::to_value(api).unwrap();
    let responses = &json["paths"]["/"]["get"]["responses"];
    let first = &responses["409"]["content"]["application/problem+json"]["schema"];
    assert_eq!(
        first,
        &responses["429"]["content"]["application/problem+json"]["schema"]
    );
    let reference = first["$ref"].as_str().unwrap();
    let component = json.pointer(reference.strip_prefix('#').unwrap()).unwrap();
    assert_eq!(component["properties"]["status"]["type"], "integer");
    assert_eq!(component["properties"]["status"]["minimum"], 100);
    assert_eq!(component["properties"]["status"]["maximum"], 999);
    let schemas = json["components"]["schemas"].to_string();
    assert!(schemas.contains("seconds"));
    assert!(schemas.contains("request"));
    assert!(schemas.contains("count"));
}

#[cfg(feature = "aide")]
#[test]
fn openapi_preserves_different_payloads_across_routes() {
    async fn retry() -> Result<(), problems::Report<TypedRetry>> {
        Err(TypedRetry {
            seconds: 30,
            request: "abc".into(),
        }
        .into_report())
    }
    async fn optional() -> Result<(), problems::Report<Optional>> {
        Err(Optional { value: Some(7) }.into_report())
    }

    for reverse in [false, true] {
        aide::generate::in_context(|context| {
            context.schema = Default::default();
        });
        let mut api = aide::openapi::OpenApi::default();
        let router = aide::axum::ApiRouter::<()>::new();
        let router = if reverse {
            router
                .api_route("/optional", aide::axum::routing::get(optional))
                .api_route("/retry", aide::axum::routing::get(retry))
        } else {
            router
                .api_route("/retry", aide::axum::routing::get(retry))
                .api_route("/optional", aide::axum::routing::get(optional))
        };
        let _router = router.finish_api(&mut api);
        let document = serde_json::to_value(api).unwrap();
        let mut references = Vec::new();
        for (path, status, fields) in [
            ("/retry", "429", &["seconds", "request"][..]),
            ("/optional", "500", &["value"][..]),
        ] {
            let reference = document["paths"][path]["get"]["responses"][status]
                ["content"]["application/problem+json"]["schema"]["$ref"]
                .as_str().unwrap();
            references.push(reference);
            let details = document
                .pointer(reference.strip_prefix('#').unwrap())
                .unwrap();
            let data_reference = details["properties"]["data"]["anyOf"][0]["$ref"]
                .as_str()
                .unwrap();
            let data = document
                .pointer(data_reference.strip_prefix('#').unwrap())
                .unwrap();
            let properties = data["properties"].as_object().unwrap();
            assert_eq!(properties.len(), fields.len());
            for field in fields {
                assert!(
                    properties.contains_key(*field),
                    "Missing {field} for {path}"
                );
            }
        }
        assert_ne!(references[0], references[1]);
    }
}
