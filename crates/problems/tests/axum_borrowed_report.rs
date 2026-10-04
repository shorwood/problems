#![cfg(all(feature = "axum", feature = "derive"))]
#[path = "support/borrowed_report.rs"]
mod support;
use axum::response::IntoResponse;

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
    let bytes = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    support::assert_body(serde_json::from_slice(&bytes).unwrap());
}
