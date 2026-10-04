# Problems

Declare public HTTP failures beside the operation that owns their meaning.
Keep internal errors in their ordinary Rust source chain. Convert a typed error
into `Report<E>` at the HTTP boundary.

```toml
problems = { path = "../problems", features = ["axum"] }
```

The runtime depends on neither SeaORM nor application initialization. It does
not interpret database failures. The application decides whether a failure is
a conflict, a missing resource, or an unexpected internal error.

## Declare and project

The `Problem` derive supplies metadata and formatted public detail. Use
`thiserror` or handwritten implementations for ordinary Rust
`Display` and `Error` behavior.

```rust
use problems::{IntoReport, Problem};

#[derive(Debug, thiserror::Error, problems::Problem)]
enum CreateFlowProblem {
    #[error("flow name already exists: {name}")]
    #[problem(
        type_uri = "urn:nanoworks:problem:flow-name-conflict",
        status = problems::StatusCode::CONFLICT,
        title = "Flow name already exists",
        detail = "A flow named '{name}' already exists."
    )]
    NameConflict {
        name: String,
    },
    #[error("failed to create flow")]
    #[problem(
        type_uri = "urn:nanoworks:problem:internal-error",
        title = "Internal server error"
    )]
    Database {
        source: std::io::Error,
    },
}

let report = CreateFlowProblem::NameConflict {
    name: "monthly-report".into(),
}.into_report();

assert_eq!(report.problem().definition().status, 409);
let body = serde_json::to_value(report.details())?;
assert!(body.get("name").is_none());
assert_eq!(body["detail"], "A flow named 'monthly-report' already exists.");
assert!(body.get("source").is_none());
# let _ = CreateFlowProblem::Database { source: std::io::Error::other("private") };
# Ok::<(), Box<dyn std::error::Error>>(())
```

An enum prefix can replace repeated type URI declarations:

```rust
use problems::{Problem, StatusCode};

#[derive(Debug, thiserror::Error, Problem)]
#[problem(prefix = "urn:example")]
enum MyProblem {
    #[error("resource not found")]
    #[problem(title = "Not Found", status = StatusCode::NOT_FOUND)]
    NotFound,
}

assert_eq!(MyProblem::NotFound.definition().type_uri, "urn:example:not-found");
```

The macro uses `heck` to convert variant identifiers to kebab case at compile
time. A colon is inserted unless the prefix already ends in `:` or `/`:
`urn:example:` and `https://example.com/problems/` both work as written.
An explicit variant `type_uri` overrides the prefix. Without a prefix, it remains
required. Duplicate resulting URIs and empty prefixes are rejected.

Renaming a variant changes its generated public problem identity. Set an explicit
`type_uri` to preserve an established identity through a Rust rename. Titles do
not influence URI generation. Status still defaults to 500 when omitted.

Titles remain static. Detail supports named fields, Rust scalar formatting
specifiers such as `{count:04x}`, and escaped braces (`{{` and `}}`). Positional
arguments and nested/dynamic format parameters are outside the initial derive
contract. Formatting a field into detail does not expose it as a separate member.

The document contains only `type`, `title`, `status`, optional `detail`, and
optional `instance`. Other error fields remain diagnostic. Source fields
(named `source`, or marked `#[source]` / `#[from]` / `#[error(source)]`) cannot be formatted
into public detail. The derive supports unit and named-field enum variants.

`Report::from(error)` and `error.into_report()` retain the same typed error.
`ProblemDetails::from(&report)` and `Report::details` construct an owned
public document without reporting the error. Definitions use `StatusCode`, so
conversion is infallible. The derive accepts status constant paths, such as `StatusCode::CONFLICT`.
Omitting `status` defaults to `StatusCode::INTERNAL_SERVER_ERROR` (500). The document remains usable after dropping the report.

## Features

The `derive` feature is enabled by default. Set `default-features = false` to
implement `Problem` manually without the procedural macro dependency. Serde
support is unconditional; the core does not depend on a JSON, YAML, or XML encoder.

| Feature | Capability |
| --- | --- |
| `derive` | `#[derive(Problem)]`, static declarations, formatted detail |
| `axum` | `IntoResponse` for `Report<E>`; enables JSON encoding |
| `actix-web` | `ResponseError` and diagnostic `Display` forwarding for `Report<E>` |
| `rocket` | `Responder` for `Report<E>` |
| `poem` | `IntoResponse` and conversion to `poem::Error` for `Report<E>` when `E: Send` |
| `salvo` | `Scribe` for `Report<E>` |
| `warp` | `Reply` for `Report<E>` when `E: Send` |
| `aide` | Declared status documentation; enables `axum` |

Pass `report.details()` to the serializer you choose, such as a YAML encoder.
The built-in HTTP integrations emit JSON. RFC XML mapping is outside this library.

## HTTP and documentation

Use a concrete error type in handlers:

```rust
# fn main() {
# #[cfg(feature = "axum")]
# {
use axum::{http::StatusCode, response::IntoResponse};
use problems::{IntoReport, Report};

#[derive(Debug, thiserror::Error, problems::Problem)]
enum CreateFlowProblem {
    #[problem(type_uri = "urn:nanoworks:problem:flow-name-conflict", status = problems::StatusCode::CONFLICT, title = "Flow name already exists")]
    #[error("name conflict")]
    NameConflict,
}

async fn create_flow() -> Result<StatusCode, Report<CreateFlowProblem>> {
    Err(CreateFlowProblem::NameConflict.into_report())
}

fn response_example() {
    let response = CreateFlowProblem::NameConflict.into_report().into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(response.headers()["content-type"], "application/problem+json");
}
# response_example();
# }
# }
```

The response uses `application/problem+json` and mirrors the HTTP status in its
numeric `status` member. Optional detail and instance are omitted when absent.
The public body contains neither `Display` text nor the internal source chain.

Poem handlers can return `Result<T, Report<E>>` when `T: poem::IntoResponse` and
`E: Problem + Send + Sync + 'static`. Poem imposes these error-branch bounds;
conversion to `poem::Error` itself needs only `E: Problem + Send`.
A `poem::Result<T>` handler can use `?` on a report-returning operation.
The conversion consumes the original error and retains its public response;
inspect or report diagnostics before converting.

Aide's `OperationOutput` implementation for `Report<E>` reads the static
declarations. It groups variants by status and lists each title and identity in
the response description. All entries reference one `ProblemDetails` schema;
only the standard members are supported. No global registry or per-variant
schemas are required.

## Framework examples

Each example serves `GET /problem` at `127.0.0.1:3000` and returns a conflict
with formatted public detail. Run one server at a time:

```sh
cargo run -p problems --example axum --features axum
cargo run -p problems --example actix-web --features actix-web
cargo run -p problems --example rocket --features rocket
cargo run -p problems --example poem --features poem
cargo run -p problems --example salvo --features salvo
cargo run -p problems --example warp --features warp
curl -i http://127.0.0.1:3000/problem
```

Axum, Actix Web, and Rocket examples return reports through `Result` handlers.
Poem and Salvo return reports directly; Warp returns them from a filter. Poem
error conversions and Warp rejection recovery remain application concerns.

Each example contains one in-process smoke test checking its route's status,
media type, and public body. For example:

```sh
cargo test -p problems --example actix-web --features actix-web
cargo test -p problems --examples --all-features
```

## Diagnostics

Use `thiserror`, `miette`, or handwritten error implementations for diagnostic
formatting and source chains. `Report<E>` retains the error and exposes it through
`Report::problem()` or `Report::into_problem()`. With `actix-web` enabled, the
wrapper implements `Display` by forwarding to the original error, as required
by Actix Web's `ResponseError` trait. The wrapper never implements `Error`.

The application decides when to log, which severity to use, and how to enrich
spans. Conversions, serialization, documentation, and HTTP response generation
emit no diagnostic events. Only explicitly declared public fields enter the
response body.

This initial library does not normalize extractor rejections, provide custom
response headers, or automatically flatten payload structs.

## Sources

- [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457.html)
- [Axum error handling](https://docs.rs/axum/latest/axum/error_handling/index.html)
- [Aide operation outputs](https://docs.rs/aide/latest/aide/operation/trait.OperationOutput.html)

- [Actix Web response errors](https://docs.rs/actix-web/latest/actix_web/error/trait.ResponseError.html)
- [Rocket responders](https://docs.rs/rocket/latest/rocket/response/trait.Responder.html)
- [Poem responses](https://docs.rs/poem/latest/poem/web/trait.IntoResponse.html)
- [Salvo scribes](https://docs.rs/salvo/latest/salvo/writing/trait.Scribe.html)
- [Warp replies](https://docs.rs/warp/latest/warp/reply/trait.Reply.html)
