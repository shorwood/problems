#![cfg(all(feature = "rocket", feature = "derive"))]
#[path = "support/borrowed_report.rs"]
mod support;
use rocket::response::Responder;

#[rocket::async_test]
async fn borrowed_response_retains_diagnostics_and_owns_body() {
    let client = rocket::local::asynchronous::Client::tracked(rocket::build())
        .await
        .unwrap();
    let request = client.get("/");
    let mut response = {
        let name = String::from("example");
        let report = support::report(&name);
        let response = (&report).respond_to(request.inner()).unwrap();
        support::assert_diagnostics(&report);
        response
    };
    assert_eq!(response.status().code, 409);
    assert_eq!(
        response.headers().get_one("content-type"),
        Some("application/problem+json")
    );
    let bytes = response.body_mut().to_bytes().await.unwrap();
    support::assert_body(serde_json::from_slice(&bytes).unwrap());
}
