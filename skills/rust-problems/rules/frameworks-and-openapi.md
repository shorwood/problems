# Frameworks + OpenAPI

## Prefer

- Use native report adapter; no custom `IntoResponse`/`ResponseError` impl or JSON envelope for same response. Leave success type native; keep error type concrete when Aide inference needed.
- Concrete `Result<T, Report<E>>` for one typed contract. Existing erased Axum response result remains valid when heterogeneous errors need no static inference; do not add enum just for syntax uniformity. See [Axum response guidance](https://docs.rs/axum/latest/axum/response/index.html).
- Add headers through framework composition; leave report's status/media type intact. Verify outer wrappers when they override either.
- Enable only needed feature. Aide already enables Axum + Schemars. Avoid direct macro dependency or payload DTO duplication for schema generation.
- Rejection adaptation at existing request boundary, only for specified failures. No app-wide middleware rewrite to make every framework rejection a problem.

Default `derive` re-exports macro; no separate macro dependency. Enable existing app framework only. `schemars`: document/payload schemas, applicable payload schema bounds. `aide`: OpenAPI inference + `axum` + `schemars`.

| Feature | Response path |
| --- | --- |
| `axum` | Owned/borrowed report `IntoResponse`; handler `Result<T, Report<E>>` |
| `actix-web` | Report `ResponseError`; converts runtime status to Actix HTTP type |
| `rocket` | Owned/borrowed report `Responder`; owned body |
| `poem` | Owned/borrowed report `IntoResponse` or conversion to `poem::Error` |
| `salvo` | Owned/borrowed report `Scribe` |
| `warp` | Owned/borrowed report `Reply`; app rejection recovery |

Responses use `application/problem+json`. Adapter `Send`/`Sync` + payload bounds vary; inspect enabled adapter before adding clones. Poem error conversion retains public response, not diagnostic error; inspect/log before consuming.

## Axum + Aide

Keep concrete `Report<E>` error return type. `ApiRouter` + `api_route` + `finish_api` infer from `E::definitions()` + outgoing data schema. Opaque response erasure loses inference. Field-derived statuses infer static 500; see [runtime status](runtime-status.md).

```rust
use aide::{axum::{ApiRouter, routing::get}, openapi::OpenApi};
use problems::{IntoReport, Problem, Report};

#[derive(Debug, thiserror::Error, Problem)]
#[error("record missing")]
#[problem(type_uri = "urn:example:missing", status = 404)]
struct Missing;

async fn fetch() -> Result<(), Report<Missing>> {
    Err(Missing.into_report())
}

let mut api = OpenApi::default();
let _app = ApiRouter::<()>::new()
    .api_route("/record", get(fetch))
    .finish_api(&mut api);
assert!(serde_json::to_value(api)?["paths"]["/record"]["get"]["responses"]
    .get("404").is_some());
# Ok::<(), serde_json::Error>(())
```

Outer composition can override HTTP status without changing body. Axum example:

```rust
use axum::{http::StatusCode, response::IntoResponse};
use problems::{IntoReport, Problem};

#[derive(Debug, thiserror::Error, Problem)]
#[error("name conflict")]
#[problem(type_uri = "urn:example:conflict", status = 409)]
struct Conflict;

let report = Conflict.into_report();
assert_eq!(report.as_details().status().as_u16(), 409);
let response = (StatusCode::UNAUTHORIZED, report).into_response();
assert_eq!(response.status(), StatusCode::UNAUTHORIZED); // Body still says 409.
```

Extractors, unmatched routes, unrelated errors do not auto-convert. Add app-owned rejection adapter when requested; preserve unhandled rejections. See checkout `examples/axum/src/lib.rs` (selective extractor adapter) or `examples/warp/src/lib.rs` (custom rejection recovery).
