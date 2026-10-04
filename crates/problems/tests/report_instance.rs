use problems::{Problem, ProblemDefinition, Report, StatusCode};

#[derive(Debug, thiserror::Error)]
#[error("private diagnostic")]
struct Failure {
    instance: Option<String>,
}

static DEFINITION: ProblemDefinition = ProblemDefinition {
    type_uri: "urn:test:failure",
    title: "Failure",
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

    fn instance(&self) -> Option<String> {
        self.instance.clone()
    }
}

fn report() -> Report<Failure> {
    Report::new(Failure { instance: None }).with_instance("urn:test:occurrence")
}

fn assert_instance(body: serde_json::Value) {
    assert_eq!(body["instance"], "urn:test:occurrence");
    assert_eq!(body["status"], 409);
}

#[test]
fn projection_preserves_fallback_and_override() {
    for (original, override_value, expected) in [
        (None, None, None),
        (Some("urn:test:original"), None, Some("urn:test:original")),
        (
            Some("urn:test:original"),
            Some("urn:test:override"),
            Some("urn:test:override"),
        ),
    ] {
        let mut report = Report::new(Failure {
            instance: original.map(String::from),
        });
        if let Some(value) = override_value {
            report = report.with_instance(value);
        }
        let borrowed = serde_json::to_value(report.as_details()).unwrap();
        let owned = serde_json::to_value(report.into_details()).unwrap();
        assert_eq!(borrowed, owned);
        assert_eq!(owned.get("instance").and_then(|v| v.as_str()), expected);
    }
}

#[test]
fn recovering_error_discards_only_report_context() {
    let original = Report::new(Failure {
        instance: Some("urn:test:original".into()),
    })
    .with_instance("urn:test:override")
    .into_problem();
    assert_eq!(original.instance.as_deref(), Some("urn:test:original"));
    assert_eq!(original.to_string(), "private diagnostic");
}

#[cfg(feature = "axum")]
#[tokio::test]
async fn axum_owned_and_borrowed_instances() {
    use axum::response::IntoResponse;
    let report = report();
    for response in [(&report).into_response(), report.into_response()] {
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        assert_instance(serde_json::from_slice(&bytes).unwrap());
    }
}

#[cfg(feature = "actix-web")]
#[actix_web::test]
async fn actix_instance() {
    use actix_web::ResponseError;
    let bytes = actix_web::body::to_bytes(report().error_response().into_body())
        .await
        .unwrap();
    assert_instance(serde_json::from_slice(&bytes).unwrap());
}

#[cfg(feature = "rocket")]
#[rocket::async_test]
async fn rocket_owned_and_borrowed_instances() {
    use rocket::response::Responder;
    let client = rocket::local::asynchronous::Client::tracked(rocket::build())
        .await
        .unwrap();
    let request = client.get("/");
    let report = report();
    for mut response in [
        (&report).respond_to(request.inner()).unwrap(),
        report.respond_to(request.inner()).unwrap(),
    ] {
        assert_instance(
            serde_json::from_slice(&response.body_mut().to_bytes().await.unwrap()).unwrap(),
        );
    }
}

#[cfg(feature = "poem")]
#[tokio::test]
async fn poem_owned_and_borrowed_instances() {
    use poem::IntoResponse;
    let report = report();
    for response in [(&report).into_response(), report.into_response()] {
        assert_instance(response.into_body().into_json().await.unwrap());
    }
}

#[cfg(feature = "salvo")]
#[tokio::test]
async fn salvo_owned_and_borrowed_instances() {
    use salvo::{Scribe, test::ResponseExt};
    let report = report();
    let mut borrowed = salvo::Response::new();
    (&report).render(&mut borrowed);
    let mut owned = salvo::Response::new();
    report.render(&mut owned);
    for mut response in [borrowed, owned] {
        assert_instance(serde_json::from_slice(&response.take_bytes(None).await.unwrap()).unwrap());
    }
}

#[cfg(feature = "warp")]
#[tokio::test]
async fn warp_owned_and_borrowed_instances() {
    use warp::Reply;
    let report = report();
    for response in [(&report).into_response(), report.into_response()] {
        let bytes = axum::body::to_bytes(axum::body::Body::new(response.into_body()), 4096)
            .await
            .unwrap();
        assert_instance(serde_json::from_slice(&bytes).unwrap());
    }
}

#[cfg(feature = "derive")]
#[test]
fn derived_problem_accepts_occurrence_context() {
    #[derive(Debug, thiserror::Error, problems::Problem)]
    #[problem(prefix = "urn:test")]
    enum Derived {
        #[error("private diagnostic")]
        #[problem(409)]
        Conflict,
    }
    let details = Report::new(Derived::Conflict)
        .with_instance("urn:test:occurrence")
        .into_details();
    assert_instance(serde_json::to_value(details).unwrap());
}

#[cfg(feature = "schemars")]
#[test]
fn instance_remains_optional_in_schema() {
    let schema = schemars::schema_for!(problems::ProblemDetails);
    let schema = serde_json::to_value(schema).unwrap();
    assert!(schema["properties"].get("instance").is_some());
    assert!(
        !schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "instance")
    );
}
