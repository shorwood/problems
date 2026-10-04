#![cfg(all(feature = "salvo", feature = "derive"))]
#[path = "support/borrowed_report.rs"]
mod support;
use salvo::{Scribe, test::ResponseExt};

#[tokio::test]
async fn borrowed_response_retains_diagnostics_and_owns_body() {
    let mut response = salvo::Response::new();
    {
        let name = String::from("example");
        let report = support::report(&name);
        (&report).render(&mut response);
        support::assert_diagnostics(&report);
    }
    assert_eq!(response.status_code.unwrap().as_u16(), 409);
    assert_eq!(
        response.headers()["content-type"],
        "application/problem+json"
    );
    let bytes = response.take_bytes(None).await.unwrap();
    support::assert_body(serde_json::from_slice(&bytes).unwrap());
}
