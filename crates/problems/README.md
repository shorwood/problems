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
}
.into_report();

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

assert_eq!(
    MyProblem::NotFound.definition().type_uri,
    "urn:example:not-found"
);
```

The derive also exposes each locally declared variant's `ProblemDefinition`
as an associated constant in SHOUTY_SNAKE_CASE: `NameConflict` becomes
`NAME_CONFLICT`. Instance lookup and the definition iterator reuse these constants.
Generated names must be distinct and cannot conflict with an enum variant or an
existing associated item. Transparent variants expose no single definition;
refer to the wrapped enum's constants instead.

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

Titles remain static. Detail supports named fields, explicit tuple indexes
(`{0}`), Rust scalar formatting specifiers such as `{count:04x}`, and escaped
braces (`{{` and `}}`). Implicit arguments (`{}`) and dynamic width or precision
are unsupported. Formatting a field into detail does not expose it as a separate member.

The document contains only `type`, `title`, `status`, optional `detail`, and
optional `instance`. Other error fields remain diagnostic. Source fields
(named `source`, or marked `#[source]` / `#[from]` / `#[error(source)]`) cannot be formatted
into public detail. The derive supports unit, named-field, and tuple enum variants. Tuple detail supports explicit positional fields (`{0}`, `{1:04x}`);
public detail cannot interpolate diagnostic source fields.

Ordinary tuple source variants retain thiserror's familiar conversion form:

```rust
use problems::{IntoReport, Problem};
use std::error::Error;

#[derive(Debug, thiserror::Error, problems::Problem)]
#[problem(prefix = "urn:example")]
enum AppProblem {
    #[error("storage failed")]
    #[problem(detail = "Unable to save the resource.")]
    Storage(#[from] std::io::Error),
}

let error = AppProblem::from(std::io::Error::other("private diagnostic"));
let report = error.into_report();
assert!(report.problem().source().is_some());
assert_eq!(AppProblem::STORAGE.status, 500);
assert_eq!(report.problem().definition(), &AppProblem::STORAGE);
assert_eq!(report.details().detail(), Some("Unable to save the resource."));
```

Public tuple fields can use explicit indexes, while source fields stay diagnostic:

```rust
use problems::Problem;

#[derive(Debug, thiserror::Error, problems::Problem)]
#[problem(prefix = "urn:example")]
enum AppProblem {
    #[error("storage failed for {1}")]
    #[problem(detail = "Unable to save {1}.")]
    Storage(#[source] std::io::Error, String),
}

let error = AppProblem::Storage(std::io::Error::other("private"), "document".into());
assert_eq!(error.detail().as_deref(), Some("Unable to save document."));
```

These variants declare their own public identity. `#[problem(transparent)]`
instead forwards the wrapped problem's metadata and occurrence data.

A struct represents a single problem without an enum wrapper. Put metadata on
its type and supply an explicit `type_uri`. Unit, named, and tuple structs use
the same detail formatting and source protection rules as enum variants.

```rust
use problems::Problem;

#[derive(Debug, thiserror::Error, problems::Problem)]
#[error("name already taken: {name}")]
#[problem(type_uri = "urn:example:name-conflict", status = 409,
          detail = "The name {name} is unavailable.")]
struct NameConflict {
    name: String,
}

let error = NameConflict { name: "alice".into() };
assert_eq!(NameConflict::DEFINITION.title, "Name Conflict");
assert_eq!(error.definition(), &NameConflict::DEFINITION);
assert_eq!(NameConflict::definitions().count(), 1);
assert_eq!(error.detail().as_deref(), Some("The name alice is unavailable."));
```

Structs expose `DEFINITION` rather than a variant constant. Enum prefixes and
`#[problem(transparent)]` forwarding remain enum features.

`Report::from(error)` and `error.into_report()` retain the same typed error.
`ProblemDetails::from(&report)` and `Report::details` construct an owned
public document without reporting the error. Definitions use `StatusCode`, so
conversion is infallible. The derive accepts only integer status literals from 100 through 999, such as `status = 409`. Constant paths and other expressions are rejected.
`ProblemDetails::status()` returns `StatusCode`, preserving status comparisons
and predicates while serialization keeps the JSON member numeric.
Omitting `status` defaults to `StatusCode::INTERNAL_SERVER_ERROR` (500). The document remains usable after dropping the report.

Attach an occurrence URI at the HTTP boundary without adding request context
to the original error:

```rust
use problems::IntoReport;

#[derive(Debug, thiserror::Error, problems::Problem)]
#[problem(prefix = "urn:example")]
enum AppProblem {
    #[error("private storage diagnostic")]
    #[problem(503)]
    Unavailable,
}

let report = AppProblem::Unavailable
    .into_report()
    .with_instance("urn:uuid:550e8400-e29b-41d4-a716-446655440000");

let details = report.into_details();
assert_eq!(details.detail(), None);
assert_eq!(
    details.instance(),
    Some("urn:uuid:550e8400-e29b-41d4-a716-446655440000")
);
let body = serde_json::to_value(details)?;
assert_eq!(body["instance"], "urn:uuid:550e8400-e29b-41d4-a716-446655440000");
# Ok::<(), serde_json::Error>(())
```

`with_instance` owns its string and overrides `Problem::instance()` for public
projection. Without an override, the error's instance is preserved; if neither
supplies one, the member is omitted. The caller supplies a valid URI reference
identifying this occurrence ([RFC 9457, section 3.1.5](https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.5)).

Read optional members with `ProblemDetails::detail()` and
`ProblemDetails::instance()`. Both return `Option<&str>` borrowed from the
document without allocation; absent members return `None`. Unlike the methods
on the `Problem` trait, these getters do not construct owned strings.

`details()` retains the report and clones an attached instance into the owned
document. `into_details()` consumes the report and moves that string. Owned
HTTP adapters use consuming projection; borrowed adapters and Actix's
`error_response(&self)` use borrowing projection. Diagnostic formatting still
comes from the original error. `into_problem()` recovers that error and discards
the attached occurrence context.

## Receiving problem documents

Use `GenericProblem` to decode owned public data from an HTTP response:

```rust
use problems::{GenericProblem, StatusCode};

let problem = {
    let body = String::from(r#"{
        "type": "urn:example:name-conflict",
        "title": "Name conflict",
        "status": 409,
        "detail": "Choose another name.",
        "instance": "/occurrences/123",
        "extension": {"ignored": true}
    }"#);
    serde_json::from_str::<GenericProblem>(&body)?
};

assert_eq!(problem.type_uri(), "urn:example:name-conflict");
assert_eq!(problem.title(), Some("Name conflict"));
assert_eq!(problem.status(), Some(StatusCode::CONFLICT));
assert_eq!(problem.detail(), Some("Choose another name."));
assert_eq!(problem.instance(), Some("/occurrences/123"));
# Ok::<(), serde_json::Error>(())
```

Compare a received type with a variant's definition without constructing its
fields or diagnostic source:

```rust
use problems::GenericProblem;

#[derive(Debug, thiserror::Error, problems::Problem)]
#[problem(prefix = "urn:example")]
enum CreateProblem {
    #[error("private diagnostic: {source}")]
    #[problem(409)]
    NameConflict { source: std::io::Error },
}

let received: GenericProblem = serde_json::from_str(
    r#"{"type":"urn:example:name-conflict"}"#,
)?;
assert!(received.is_type(&CreateProblem::NAME_CONFLICT));
# Ok::<(), serde_json::Error>(())
```

`is_type` compares only the type URI. Title, status, detail, instance, and
private diagnostics do not affect this identity check. Relative received URI
references require resolution before comparing them with declared identities.

Both document types implement `PartialEq` and `Eq`. Equality compares all five
public members, including `instance`, and works between `GenericProblem` and
`ProblemDetails` in either direction. For repeated failures, use
`received.matches(&report.details())`: it compares type, title, status, and detail
while ignoring only `instance`. Missing members are not wildcards; an omitted
detail differs from an empty string. `is_type()` remains the identity-only check.

The document owns its strings and remains usable after dropping the response
buffer. Missing `type` defaults to `about:blank`; other missing members return
`None`. Unknown extensions are discarded. Decoding uses ordinary Serde type
checks: wrongly typed members fail decoding. This intentionally does not implement
[RFC 9457's tolerant member-processing rule](https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1).
Status is decoded as an optional `StatusCode`; invalid values fail decoding. Relative
URI references remain unresolved; the application must use the response's base
URI when interpreting them.

`GenericProblem::from(report.into_details())` also converts a local public
document: it owns the static type and title and moves detail and instance.
This receiving type carries public data only. It has no `Problem`, `Error`,
framework response, or Aide operation implementation, and cannot recover the
original diagnostic error or its static definitions. Keep `Report<E>` in server
handlers and use the actual HTTP response status when handling received errors;
the body's status is advisory.

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
| `schemars` | `JsonSchema` for producer and receiving documents |
| `aide` | Declared status documentation; enables `axum` and `schemars` |

Enable `schemars` to generate schemas for `ProblemDetails` and `GenericProblem`
without an HTTP framework. Producer schemas require type, title, and status;
receiving schemas reflect missing members and the `about:blank` default.
`Report<E>` uses its projected document through Aide; declaration metadata and
private diagnostic errors are not response schemas.

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
    #[problem(
        type_uri = "urn:example:read-failed",
        title = "Internal server error"
    )]
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
    #[problem(
        type_uri = "urn:nanoworks:problem:flow-name-conflict",
        status = 409,
        title = "Flow name already exists"
    )]
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
    let report = CreateFlowProblem::NameConflict.into_report();
    let response = (&report).into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(response.headers()["content-type"], "application/problem+json");
    assert_eq!(report.problem().to_string(), "name conflict");
}
# response_example();
# }
# }
```

The response uses `application/problem+json` and mirrors the HTTP status in its
numeric `status` member. Optional detail and instance are omitted when absent.
The public body contains neither `Display` text nor the internal source chain.

The origin server must send the same HTTP status as the document's `status`
member ([RFC 9457, section 3.1.2](https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.2)).
The adapters use the problem definition for both. If the application's
classification changes, choose a problem with the appropriate declared status.

In Axum, add headers without supplying an outer status:

```rust
# fn main() {
# #[cfg(feature = "axum")]
# {
use axum::response::IntoResponse;
use problems::IntoReport;

#[derive(Debug, thiserror::Error, problems::Problem)]
#[problem(prefix = "urn:example")]
enum AppProblem {
    #[error("private storage diagnostic")]
    #[problem(503)]
    Unavailable,
}

let report = AppProblem::Unavailable.into_report();
let response = ([("retry-after", "60")], report).into_response();
assert_eq!(response.status().as_u16(), 503);
assert_eq!(response.headers()["retry-after"], "60");
# }
# }
```

An outer `(StatusCode::UNAUTHORIZED, report)` instead replaces the HTTP status
with 401 while leaving the document's declared status unchanged. Subsequent
middleware can also cause a mismatch. The report adapter cannot enforce
consistency after its response has been composed or modified. RFC 9457 separately
allows for intermediaries to change the transmitted status; that does not excuse
an origin application from generating matching values.
See [Axum response composition](https://docs.rs/axum/latest/axum/response/index.html).

Axum, Rocket, Poem, Salvo, and Warp accept borrowed reports as well as owned
reports. Borrowed rendering produces an owned public response and leaves the
original report available for diagnostic inspection, without requiring `Clone`.
Render the response before dropping a local report; returning a reference to a
handler-local report is not possible. Actix's `error_response(&self)` already
borrows its report.

Poem and Warp require `E: Sync` for their borrowed response implementations,
compared with `E: Send` for owned reports. Salvo's borrowed `Scribe` itself adds
no bound, but its async `Writer` integration requires `E: Sync`.

Poem handlers can return `Result<T, Report<E>>` when `T: poem::IntoResponse` and
`E: Problem + Send + Sync + 'static`. Poem imposes these error-branch bounds;
conversion to `poem::Error` itself needs only `E: Problem + Send`.
A `poem::Result<T>` handler can use `?` on a report-returning operation.
The conversion consumes the original error and retains its public response;
inspect or report diagnostics before converting.
Alternatively, `poem::Error::from(&report)` requires `E: Problem + Sync` and
leaves the original report available. The resulting Poem error contains only
the public response and can outlive the report; it does not retain its source chain.

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

### Framework error boundaries

Adapters render reports explicitly produced by the application. They do not
intercept extractor rejections, middleware responses, or missing routes. For
example, Axum normally rejects malformed JSON with a plain-text 400 response
before calling a handler that takes `Json<T>`.

Define an application-owned `ProblemJson<T>` extractor to share an explicit
classification policy across handlers. Delegate parsing to Axum’s `Json<T>`,
keep framework diagnostics in the source chain, and choose the public explanation:

```rust
# fn main() {
# #[cfg(feature = "axum")]
# {
use axum::{
    Json,
    extract::{FromRequest, Request, rejection::JsonRejection},
    response::{IntoResponse, Response},
};
use problems::IntoReport;

#[derive(Debug, thiserror::Error, problems::Problem)]
enum RequestProblem {
    #[error("JSON extraction failed: {source}")]
    #[problem(
        type_uri = "urn:example:malformed-json",
        status = 400,
        title = "Malformed JSON",
        detail = "The request body must contain valid JSON."
    )]
    MalformedJson { source: JsonRejection },
}

struct ProblemJson<T>(T);

impl<T, S> FromRequest<S> for ProblemJson<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(request, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(source @ JsonRejection::JsonSyntaxError(_)) => Err(
                RequestProblem::MalformedJson { source }
                    .into_report()
                    .into_response(),
            ),
            Err(rejection) => Err(rejection.into_response()),
        }
    }
}

async fn create(ProblemJson(value): ProblemJson<serde_json::Value>) -> Json<serde_json::Value> {
    Json(value)
}

# let _router = axum::Router::<()>::new().route("/create", axum::routing::post(create));
# }
# }
```

This recipe changes only JSON syntax failures. Other extraction rejections,
such as missing JSON content type, retain their framework responses. Delegating
to `Json<T>` also preserves Axum’s body limits. Handlers opt in by taking
`ProblemJson<T>` as their final parameter because it consumes the request body.
This type belongs to the application, not the library. The Axum example compares
`POST /json` with `POST /json-problem`.

See [Axum’s custom extractor guidance](https://docs.rs/axum/latest/axum/extract/index.html#customizing-extractor-responses).

Router fallbacks and middleware require their own explicit classification at
the framework's handling points. The same principle applies to all adapters:
choose a declared problem where the failure is handled, then render its report.

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

fn authenticate() -> Result<(), AuthProblem> {
    Ok(())
}

fn store() -> Result<(), StorageProblem> {
    Ok(())
}

fn operation() -> Result<(), HandlerProblem> {
    authenticate()?;
    store()?;
    Ok(())
}

async fn handler() -> Result<(), Report<HandlerProblem>> {
    operation().map_err(IntoReport::into_report)
}

assert_eq!(
    HandlerProblem::definitions()
        .map(|definition| definition.status.as_u16())
        .collect::<Vec<_>>(),
    [401, 503]
);
```

`#[error(transparent)]` controls diagnostic forwarding; `#[problem(transparent)]`
controls public problem forwarding. Transparent variants cannot also declare
status, title, type URI, or detail. An enclosing prefix only affects locally
declared variants. Aide reads all wrapped definitions without constructing errors.

`Problem::definitions()` now returns an iterator of static definition references.
Manual implementations should return a slice's `.iter()` or chain other problem
iterators; callers that need indexing can collect the references into a `Vec`.

### Warp rejection recovery

Warp applications can wrap a report in an application-owned rejection and
recover it through a borrowed reply. The wrapper's problem type must satisfy
`Send + Sync + 'static`; it does not need `Clone`.

```rust
# fn main() {
# #[cfg(feature = "warp")]
# {
use problems::{IntoReport, Report};
use warp::{Filter, Rejection, Reply};

#[derive(Debug, thiserror::Error, problems::Problem)]
#[problem(prefix = "urn:example")]
enum AppProblem {
    #[error("private storage failure: {source}")]
    #[problem(503)]
    Unavailable { source: std::io::Error },
}

#[derive(Debug)]
struct AppRejection(Report<AppProblem>);
impl warp::reject::Reject for AppRejection {}

async fn create() -> Result<(), AppProblem> {
    Err(AppProblem::Unavailable {
        source: std::io::Error::other("private diagnostic"),
    })
}

async fn create_handler() -> Result<&'static str, Rejection> {
    create()
        .await
        .map_err(|problem| warp::reject::custom(AppRejection(problem.into_report())))?;
    Ok("created")
}

async fn recover_problem(
    rejection: Rejection,
) -> Result<warp::reply::Response, Rejection> {
    if let Some(problem) = rejection.find::<AppRejection>() {
        return Ok((&problem.0).into_response());
    }
    Err(rejection)
}

let routes = warp::path("create")
    .and(warp::path::end())
    .and(warp::post())
    .and_then(create_handler)
    .recover(recover_problem);
# let _ = routes;
# }
# }
```

Recovery renders an owned response while borrowing the stored report. It leaves
unrelated rejections for Warp or subsequent application recovery to handle.
Apply recovery after combining routes when alternative routes should be tried
before an application rejection is turned into a response. Inspect diagnostics
inside recovery before returning the response if the application needs them.
The library implements `Reply` for owned and borrowed reports; applications own
their `Reject` wrapper and classification policy.

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
