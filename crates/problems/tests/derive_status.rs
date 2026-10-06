#![cfg(feature = "derive")]

use problems::{IntoReport, Problem, StatusCode};

#[derive(Debug, thiserror::Error, Problem)]
#[error("private failure")]
#[problem(type_uri = "urn:test:dynamic")]
struct Dynamic<T> {
    #[problem(status)]
    code: T,
}

#[derive(Debug, thiserror::Error, Problem)]
#[problem(prefix = "urn:test")]
enum Failure {
    #[error("private failure")]
    #[problem("I'm a teapot, not a {thing}")]
    #[problem(status = 418)]
    Teapot { thing: String },
    #[error("private failure")]
    #[problem(r#"{{value}} {1:04x}"#)]
    Tuple(bool, u16),
    #[error("private failure")]
    Passthrough(#[problem(status, data = "code")] u32),
    #[error("private failure")]
    Named {
        #[problem(status)]
        code: u32,
    },
    #[error(transparent)]
    #[problem(transparent)]
    Wrapped(Dynamic<u32>),
}

#[test]
fn detail_shorthand_reuses_formatting() {
    let problem = Failure::Teapot {
        thing: "coffee machine".into(),
    };
    assert_eq!(
        problem.detail().as_deref(),
        Some("I'm a teapot, not a coffee machine")
    );
    assert_eq!(problem.status(), StatusCode::IM_A_TEAPOT);
    let tuple = Failure::Tuple(false, 42);
    assert_eq!(tuple.detail().as_deref(), Some("{value} 002a"));
    assert_eq!(tuple.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn checked_conversion_covers_integer_widths_and_boundaries() {
    macro_rules! check {
        ($ty:ty, $($value:expr => $expected:expr),+ $(,)?) => {
            $(assert_eq!(Dynamic { code: $value as $ty }.status().as_u16(), $expected);)+
        };
    }
    check!(i8, -1 => 500, 99 => 500, 100 => 100, i8::MAX => 127);
    check!(u8, 0 => 500, 99 => 500, 100 => 100, u8::MAX => 255);
    macro_rules! wide {
        ($($ty:ty),+) => {$(check!($ty, 99 => 500, 100 => 100, 418 => 418, 599 => 599,
            600 => 600, 999 => 999, 1000 => 500, <$ty>::MAX => 500);)+};
    }
    wide!(i16, i32, i64, u16, u32, u64);
    check!(i16, i16::MIN => 500, -1 => 500);
    check!(i32, i32::MIN => 500, -1 => 500, 65536 + 418 => 500);
    check!(i64, i64::MIN => 500, -1 => 500, 65536 + 418 => 500);
    check!(u32, 65536 + 418 => 500);
    check!(u64, 65536 + 418 => 500);
    assert_eq!(
        Dynamic {
            code: StatusCode::NOT_FOUND
        }
        .status(),
        StatusCode::NOT_FOUND
    );
    type Code = u32;
    assert_eq!(Dynamic { code: 429 as Code }.status().as_u16(), 429);
}

#[test]
fn runtime_status_reaches_both_documents_and_transparent_wrappers() {
    for code in [429, 99, 999, u32::MAX] {
        let expected = if (100..=999).contains(&code) {
            code
        } else {
            500
        };
        for problem in [Failure::Wrapped(Dynamic { code }), Failure::Named { code }] {
            let report = problem.into_report();
            assert_eq!(report.problem().definition().status.as_u16(), 500);
            assert_eq!(Failure::definitions().last().unwrap().status.as_u16(), 500);
            assert_eq!(
                serde_json::to_value(report.as_details()).unwrap()["status"],
                expected
            );
            assert_eq!(
                serde_json::to_value(report.into_details()).unwrap()["status"],
                expected
            );
        }
    }
    let details = Failure::Passthrough(409).into_report().into_details();
    assert_eq!(details.status().as_u16(), 409);
    assert_eq!(serde_json::to_value(details).unwrap()["data"]["code"], 409);
    assert!(
        Dynamic { code: 418u32 }
            .into_report()
            .as_details()
            .data()
            .is_none()
    );
}

#[cfg(feature = "axum")]
#[tokio::test]
async fn axum_runtime_status_matches_body() {
    use axum::response::IntoResponse;
    for code in [429, 99] {
        let report = Dynamic { code }.into_report();
        for response in [(&report).into_response(), report.into_response()] {
            let status = response.status().as_u16();
            assert_eq!(status, if code == 429 { 429 } else { 500 });
            let bytes = axum::body::to_bytes(response.into_body(), 4096)
                .await
                .unwrap();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["status"],
                status
            );
        }
    }
}

#[cfg(feature = "actix-web")]
#[tokio::test]
async fn actix_runtime_status_matches_body() {
    use actix_web::ResponseError;
    let report = Dynamic { code: 429u32 }.into_report();
    assert_eq!(report.status_code().as_u16(), 429);
    let response = report.error_response();
    assert_eq!(response.status().as_u16(), 429);
    let bytes = actix_web::body::to_bytes(response.into_body())
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["status"],
        429
    );
}

#[cfg(feature = "poem")]
#[tokio::test]
async fn poem_runtime_status_matches_body() {
    use poem::IntoResponse;
    let report = Dynamic { code: 429u32 }.into_report();
    let borrowed_error: poem::Error = (&report).into();
    let owned_error: poem::Error = Dynamic { code: 429u32 }.into_report().into();
    for response in [
        (&report).into_response(),
        report.into_response(),
        borrowed_error.into_response(),
        owned_error.into_response(),
    ] {
        assert_eq!(response.status().as_u16(), 429);
        assert_eq!(
            response
                .into_body()
                .into_json::<serde_json::Value>()
                .await
                .unwrap()["status"],
            429
        );
    }
}

#[cfg(feature = "salvo")]
#[tokio::test]
async fn salvo_runtime_status_matches_body() {
    use salvo::{Scribe, test::ResponseExt};
    let report = Dynamic { code: 429u32 }.into_report();
    let mut borrowed = salvo::Response::new();
    (&report).render(&mut borrowed);
    let mut owned = salvo::Response::new();
    report.render(&mut owned);
    for mut response in [borrowed, owned] {
        assert_eq!(response.status_code.unwrap().as_u16(), 429);
        let bytes = response.take_bytes(None).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["status"],
            429
        );
    }
}

#[cfg(feature = "warp")]
#[tokio::test]
async fn warp_runtime_status_matches_body() {
    use warp::Reply;
    let report = Dynamic { code: 429u32 }.into_report();
    for response in [(&report).into_response(), report.into_response()] {
        assert_eq!(response.status().as_u16(), 429);
        let bytes = axum::body::to_bytes(axum::body::Body::new(response.into_body()), 4096)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["status"],
            429
        );
    }
}

#[cfg(feature = "rocket")]
#[rocket::async_test]
async fn rocket_runtime_status_matches_body() {
    use rocket::response::Responder;
    let client = rocket::local::asynchronous::Client::tracked(rocket::build())
        .await
        .unwrap();
    let request = client.get("/");
    let report = Dynamic { code: 429u32 }.into_report();
    for mut response in [
        (&report).respond_to(request.inner()).unwrap(),
        report.respond_to(request.inner()).unwrap(),
    ] {
        assert_eq!(response.status().code, 429);
        let bytes = response.body_mut().to_bytes().await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["status"],
            429
        );
    }
}
