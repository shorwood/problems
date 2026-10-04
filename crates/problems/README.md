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
        status = 409,
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
use problems::Problem;

#[derive(Debug, thiserror::Error, Problem)]
#[problem(prefix = "urn:example")]
enum MyProblem {
    #[error("resource not found")]
    #[problem(404)]
    NotFound,
}

assert_eq!(MyProblem::NotFound.definition().type_uri, "urn:example:not-found");
```

The macro uses `heck` to convert variant identifiers to kebab case at compile
time. A colon is inserted unless the prefix already ends in `:` or `/`:
`urn:example:` and `https://example.com/problems/` both work as written.
An explicit variant `type_uri` overrides the prefix. Without a prefix, it remains
required. Duplicate resulting URIs and empty prefixes are rejected.

Titles default to the variant name converted to Title Case by `heck`:
`NameConflict` becomes `Name Conflict`. An explicit nonempty `title` overrides
the default. Renaming a variant also changes its default title.

`#[problem(409)]` is shorthand for `#[problem(status = 409)]`. Both forms accept
only numeric literals from 100 through 999 and share duplicate-status checks.
Use a separate `#[problem(...)]` attribute for an explicit `type_uri`, `title`,
or `detail` when using shorthand. An enum prefix can supply the type URI, as above.

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
conversion is infallible. The derive accepts only integer status literals from 100 through 999, such as `status = 409`. Constant paths and other expressions are rejected.
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

Keep application operations returning their typed error, then wrap it in a
report at the handler boundary:

```rust
use problems::Report;
use std::path::Path;

#[derive(Debug, thiserror::Error, problems::Problem)]
enum AppProblem {
    #[error("failed to read file: {cause}")]
    #[problem(type_uri = "urn:example:read-failed", title = "Internal server error")]
    ReadFailed {
        #[from]
        cause: std::io::Error,
    },
}

fn read_file(path: &Path) -> Result<Vec<u8>, AppProblem> {
    Ok(std::fs::read(path)?)
}

fn handler(path: &Path) -> Result<Vec<u8>, Report<AppProblem>> {
    Ok(read_file(path)?)
}

// When the handler calls the lower-level API directly, classify its error first.
fn direct_handler(path: &Path) -> Result<Vec<u8>, Report<AppProblem>> {
    Ok(std::fs::read(path).map_err(AppProblem::from)?)
}

# // Reading a directory exercises a real I/O failure without creating a file.
# for report in [handler(Path::new(".")).unwrap_err(), direct_handler(Path::new(".")).unwrap_err()] {
#     assert!(std::error::Error::source(report.problem()).is_some());
#     let body = serde_json::to_value(report.details()).unwrap();
#     assert_eq!(body, serde_json::json!({
#         "type": "urn:example:read-failed",
#         "title": "Internal server error",
#         "status": 500
#     }));
# }
```

For `Result`, `?` uses one `From` conversion. It does not chain
`io::Error -> AppProblem -> Report<AppProblem>`. The operation above uses
thiserror's `From<io::Error>` implementation; the handler uses the existing
`From<AppProblem> for Report<AppProblem>`. The direct handler makes the first
conversion explicit with `map_err(AppProblem::from)`.

`IntoReport` is an extension trait on the original error: it adds
`.into_report()`, which returns `Report<E>`. Framework response traits are
implemented on that report. A return type such as `Result<T, impl IntoReport>`
only promises conversion and does not satisfy Axum's response contract.

Use a concrete report error type in handlers:

```rust
# fn main() {
# #[cfg(feature = "axum")]
# {
use axum::{http::StatusCode, response::IntoResponse};
use problems::{IntoReport, Report};

#[derive(Debug, thiserror::Error, problems::Problem)]
enum CreateFlowProblem {
    #[problem(type_uri = "urn:nanoworks:problem:flow-name-conflict", status = 409, title = "Flow name already exists")]
    #[error("name conflict")]
    NameConflict,
}

async fn create_flow() -> Result<StatusCode, Report<CreateFlowProblem>> {
    Err(CreateFlowProblem::NameConflict.into_report())
}

// Opaque Axum responses work after explicit conversion.
async fn opaque_create_flow() -> impl IntoResponse {
    create_flow().await.into_response()
}

# let _router = axum::Router::<()>::new()
#     .route("/concrete", axum::routing::get(create_flow))
#     .route("/opaque", axum::routing::get(opaque_create_flow));

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

Declarations use numeric literals, including in Actix applications:
`#[problem(status = 409, ...)]`. No framework status import is needed for a
declaration. Runtime definitions retain `problems::StatusCode` (`http` 1.x);
the Actix adapter converts it numerically to Actix's `http` 0.2 status type.

Aide's `OperationOutput` implementation for `Report<E>` reads the static
declarations. It groups variants by status and lists each title and identity in
the response description. All entries reference one `ProblemDetails` schema;
only the standard members are supported. No global registry or per-variant
schemas are required.

Keep `Result<T, Report<E>>` when Aide needs these declared statuses. Returning
only `impl IntoResponse` hides the report's `OperationOutput` implementation;
the Axum response bound alone does not expose OpenAPI metadata.

## Framework examples

### Combining problem enums

A handler can combine existing problem enums through a boundary enum. Mark
single-field tuple variants with `#[problem(transparent)]` to forward public
metadata, detail, and instance without repeating their declarations:

```rust
use problems::{IntoReport, Problem, Report};

#[derive(Debug, thiserror::Error, Problem)]
#[problem(prefix = "urn:auth")]
enum AuthProblem {
    #[error("private authentication diagnostic")]
    #[problem(401)]
    Unauthorized,
}

#[derive(Debug, thiserror::Error, Problem)]
#[problem(prefix = "urn:storage")]
enum StorageProblem {
    #[error("private storage diagnostic")]
    #[problem(503)]
    Unavailable,
}

#[derive(Debug, thiserror::Error, Problem)]
enum HandlerProblem {
    #[error(transparent)]
    #[problem(transparent)]
    Auth(#[from] AuthProblem),
    #[error(transparent)]
    #[problem(transparent)]
    Storage(#[from] StorageProblem),
}

fn authenticate() -> Result<(), AuthProblem> { Ok(()) }
fn store() -> Result<(), StorageProblem> { Ok(()) }

fn operation() -> Result<(), HandlerProblem> {
    authenticate()?;
    store()?;
    Ok(())
}

async fn handler() -> Result<(), Report<HandlerProblem>> {
    operation().map_err(IntoReport::into_report)
}

assert_eq!(HandlerProblem::definitions().map(|d| d.status.as_u16())
    .collect::<Vec<_>>(), [401, 503]);
```

`#[error(transparent)]` controls diagnostic forwarding; `#[problem(transparent)]`
controls public problem forwarding. Transparent variants cannot also declare
status, title, type URI, or detail. An enclosing prefix only affects locally
declared variants. Aide reads all wrapped definitions without constructing errors.

`Problem::definitions()` now returns an iterator of static definition references.
Manual implementations should return a slice's `.iter()` or chain other problem
iterators; callers that need indexing can collect the references into a `Vec`.

### Running examples

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
