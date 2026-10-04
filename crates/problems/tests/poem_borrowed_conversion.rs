#![cfg(all(feature = "poem", feature = "derive"))]
#[path = "support/borrowed_report.rs"]
mod support;

#[tokio::test]
async fn borrowed_conversion_retains_diagnostics_and_owns_response() {
    let error = {
        let name = String::from("example");
        let report = support::report(&name);
        let error = poem::Error::from(&report);
        support::assert_diagnostics(&report);
        error
    };
    assert!(error.is_from_response());
    assert!(!error.has_source());
    let response = error.into_response();
    assert_eq!(response.status(), 409);
    assert_eq!(
        response.headers()["content-type"],
        "application/problem+json"
    );
    support::assert_body(response.into_body().into_json().await.unwrap());
}
