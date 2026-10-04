# Problems

Typed Rust errors with explicit public HTTP responses using
[RFC 9457 Problem Details](https://www.rfc-editor.org/rfc/rfc9457.html).
Declare metadata, client-facing detail, and structured data beside the error.
Keep its diagnostic message and source chain available for logging.
The same declaration supplies Axum responses and Aide's OpenAPI metadata.

```rust
use aide::axum::{ApiRouter, routing::post};
use problems::{IntoReport, Report};

// --- Declare diagnostics and the public contract.
#[derive(Debug, thiserror::Error, problems::Problem)]
#[problem(prefix = "urn:example")]
enum CreateProblem {
    #[error("duplicate name: {name}")]
    #[problem(status = 409, detail = "The name '{name}' is already in use.")]
    NameConflict {
        #[problem(data)]
        name: String,
    },
}

// --- Return a report when the operation fails.
async fn create_flow() -> Result<(), Report<CreateProblem>> {
    Err(CreateProblem::NameConflict { name: "monthly".into() }.into_report())
}

// --- Register the Axum handler and generate its OpenAPI responses.
let mut api = aide::openapi::OpenApi::default();
let app = ApiRouter::<()>::new()
    .api_route("/flows", post(create_flow))
    .finish_api(&mut api);
```

With the router served on `localhost:3000`:

```sh
curl -i -X POST http://localhost:3000/flows
```

Expected response:

```http
HTTP/1.1 409 Conflict
Content-Type: application/problem+json

{
  "type": "urn:example:name-conflict",
  "title": "Name Conflict",
  "status": 409,
  "detail": "The name 'monthly' is already in use.",
  "data": { "name": "monthly" }
}
```

Aide generates these OpenAPI responses for `POST /flows`:

```json
{
  "200": { "description": "no content" },
  "409": {
    "description": "Name Conflict\nProblem type: urn:example:name-conflict",
    "content": {
      "application/problem+json": {
        "schema": {
          "$ref": "#/components/schemas/ProblemDetails_for_CreateProblemData_for_string"
        }
      }
    }
  }
}
```

The referenced component schema includes the public document and its typed
`data` payload. The 200 response comes from the handler's `()` success type.

## Setup

This repository contains the `problems` runtime crate and the `problems-derive`
procedural macro crate. They are not published to crates.io yet. After cloning
the repository into `~/Workspaces/problems`, use a local path:

```toml
[dependencies]
problems = { path = "../problems/crates/problems", features = ["aide"] }
axum = "0.8"
aide = { version = "0.15", features = ["axum"] }
thiserror = "2"
```

## Features

| Feature | Provides |
| --- | --- |
| `derive` (default) | `#[derive(Problem)]` for structs and enums |
| `axum` | Axum responses with the declared status and `application/problem+json` |
| `actix-web` | Actix Web responses with the declared status and `application/problem+json` |
| `rocket` | Rocket responses with the declared status and `application/problem+json` |
| `poem` | Poem responses with the declared status and `application/problem+json` |
| `salvo` | Salvo responses with the declared status and `application/problem+json` |
| `warp` | Warp responses with the declared status and `application/problem+json` |
| `schemars` | Schemas for documents and generated public payloads |
| `aide` | OpenAPI response schemas and declared statuses; enables `axum` and `schemars` |

Schema support uses Schemars 1.2. Aide 0.15 still depends on Schemars 0.9;
the integration transfers generated schemas and component definitions through
their shared JSON representation. Schemars 0.9 remains a transitive dependency.

## API

- `Problem` defines what clients see: type, title, status, detail, and data.
- `Report` wraps the error for an HTTP response while keeping it available
  through `problem()` for diagnostics.
- `ProblemDetails` is the outgoing document. Use `as_details()` to borrow its
  data or `into_details()` to move it out of the report.
- `ProblemDocument` reads an incoming problem document with Serde.

Fields are public only when referenced in `detail` or selected with
`#[problem(data)]`. Diagnostic sources stay private. Optional document members
are omitted when absent.

## Integration

Return `Result<T, Report<E>>` from Axum handlers. Keep the concrete `Report<E>`
return type when using Aide so it can read the response declarations.
See the [runnable Axum example](crates/problems/examples/axum.rs).

The application classifies failures and handles framework rejections. Creating
or rendering a report does not log errors or automatically convert unrelated
failures into problems.

See [runtime documentation](crates/problems/src/lib.rs), [derive documentation](crates/problems-derive/src/lib.rs),
and [framework examples](crates/problems/examples/) for the full API and attribute grammar.

## Contributing

Keep changes focused and cover changed behavior. Enable the relevant feature
when changing an integration.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo test -p problems --no-default-features --locked
cargo deny --all-features check
```

The GitHub Actions CI runs these checks on pushes and pull requests using stable
Rust. Install the dependency checker with `cargo install cargo-deny --locked`.
The committed lockfile records the tested dependency versions.

`deny.toml` records one targeted maintenance advisory exception for
[`proc-macro-error2`](https://rustsec.org/advisories/RUSTSEC-2026-0173).
Its latest stable release is unmaintained; replacing the diagnostic layer is
deferred from this extraction.

## License

Licensed under the [MIT license](LICENSE).
