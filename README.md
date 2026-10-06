# Problems

Typed Rust errors with explicit public HTTP responses using
[RFC 9457 Problem Details](https://www.rfc-editor.org/rfc/rfc9457.html).
Declare metadata, client-facing detail, and structured data beside the error.
Keep its diagnostic message and source chain available for logging.
The same declaration supplies Axum responses and Aide's OpenAPI metadata.

```rust
use aide::axum::{ApiRouter, routing::post};
use aide::openapi::OpenApi;
use problems::{Problem, Report};
use thiserror::Error;

// --- Declare
#[derive(Debug, Error, Problem)]
#[problem(prefix = "urn:tea")]
enum BrewProblem {
    #[error("teapot {serial} received an order for {drink}")]
    #[problem(
        status = 418,
        title = "I'm a teapot",
        detail = "You ordered '{drink}'. I boil leaves. Manage your expectations."
    )]
    CoffeeRequested {
        #[problem(data)]
        drink: String,        // Public.
        serial: &'static str, // Internal diagnostic.
    },
}

// --- A simple operation that fails when coffee is requested.
async fn brew(drink: &str) -> Result<(), BrewProblem> {
    if drink == "tea" {
        return Ok(());
    }
    Err(BrewProblem::CoffeeRequested {
        drink: drink.into(),
        serial: "PRIVATE-TEAPOT-007",
    })
}

// --- Define a helper that returns a report for the Axum handler.
async fn order_coffee() -> Result<(), Report<BrewProblem>> {
    brew("espresso").await?;
    Ok(())
}

// --- The same declaration supplies the response and OpenAPI schema.
let mut api = OpenApi::default();
let app = ApiRouter::<()>::new()
    .api_route("/coffee", post(order_coffee))
    .finish_api(&mut api);
```

The handler uses `?` to convert `BrewProblem` into `Report<BrewProblem>`.
The serial stays available through `report.problem()` for diagnostics, while
only the selected `drink` field appears in the public payload and its schema.

With the router served on `localhost:3000` (variable headers omitted and JSON
formatted for readability):

```sh
$ curl -i -X POST http://localhost:3000/coffee
HTTP/1.1 418 I'm a teapot
content-type: application/problem+json

{
  "type": "urn:tea:coffee-requested",
  "title": "I'm a teapot",
  "status": 418,
  "detail": "You ordered 'espresso'. I boil leaves. Manage your expectations.",
  "data": { "drink": "espresso" }
}
```

Aide generates these OpenAPI responses for `POST /coffee`:

```json
{
  "200": { "description": "no content" },
  "418": {
    "description": "I'm a teapot\nProblem type: urn:tea:coffee-requested",
    "content": {
      "application/problem+json": {
        "schema": {
          "$ref": "#/components/schemas/ProblemDetails_for_BrewProblemData"
        }
      }
    }
  }
}
```

The referenced component schema includes the public document and its typed
`data` payload. The 200 response comes from the handler's `()` success type.

## Setup

Add `problems` to your `Cargo.toml`. Enable the `aide` feature to use the
Axum and OpenAPI integration shown above:

```toml
[dependencies]
problems = { version = "0.1", features = ["aide"] }
axum = "0.8"
aide = { version = "0.15", features = ["axum"] }
thiserror = "2"
```

The default `derive` feature includes `problems-derive` and re-exports
`#[derive(Problem)]`; no separate macro dependency is needed. Enable only the
framework features your application uses.

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
See the [runnable Axum example](examples/axum/src/lib.rs).

The application classifies failures and handles framework rejections. Creating
or rendering a report does not log errors or automatically convert unrelated
failures into problems.

See [runtime documentation](https://docs.rs/problems),
[derive documentation](https://docs.rs/problems-derive),
and [framework examples](examples/) for the full API and attribute grammar.

## Contributing

The contribution environment uses Nix and direnv on x86_64 Linux. The
committed flake and lockfile provide Rust 1.99.0, rustfmt, Clippy, cargo-deny,
and Just.
Both crates use Rust edition 2024.

Install Nix with flakes enabled, install direnv, and
[enable its shell hook](https://direnv.net/docs/hook.html). Then clone the
repository and allow its environment:

```sh
git clone https://github.com/shorwood/problems.git
cd problems
direnv allow
```

Direnv loads the development shell automatically when you enter the repository.
You can also enter it explicitly with `nix develop`.

Keep changes focused and cover changed behavior. Enable the relevant feature
when changing an integration. Run these checks in the development shell:

```sh
just check
```

`just check` checks formatting, runs Clippy and workspace tests with all features,
tests the runtime without default features, and checks dependencies with cargo-deny.
GitHub Actions runs the same recipe inside the pinned Nix shell. The committed
Cargo lockfile records the tested dependency versions;
`flake.lock` pins the Nix development environment.

## License

Licensed under the [MIT license](LICENSE).
