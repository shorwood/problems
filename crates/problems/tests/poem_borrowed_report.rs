#![cfg(all(feature = "poem", feature = "derive"))]
#[path = "support/borrowed_report.rs"]
mod support;
use poem::IntoResponse;

#[tokio::test]
async fn borrowed_response_retains_diagnostics_and_owns_body() {
    let response = {
        let name = String::from("example");
        let report = support::report(&name);
        let response = (&report).into_response();
        support::assert_diagnostics(&report);
        response
    };
    assert_eq!(response.status(), 409);
    assert_eq!(
        response.headers()["content-type"],
        "application/problem+json"
    );
    support::assert_body(response.into_body().into_json().await.unwrap());
}
