# Declarations

## Prefer

- Derive ordinary contracts; manual impl only when derive cannot express required behavior. One problem shape → struct; related public cases → small enum. No catchall enum solely to collect every app error.
- Named fields for contextual values; unit case when no values needed. Tuple wrapper fits transparent delegation.
- Omit title when generated title fits. Use status/detail shorthand when readable; group named metadata when several options belong together. Preserve project style.
- Prefix for intentional name-based identities. Existing published URI → preserve explicitly during rename; avoid accidental contract change.
- Inner error already has correct public contract → transparent wrapper + `#[from]` at aggregation boundary. Different public meaning → local declaration with private source. No repeated metadata match beside derive.

`Problem` requires `Error`; derive supplies neither `Display` nor `Error`. Use existing diagnostic implementation, e.g. `thiserror`.

- Struct: explicit `type_uri`. Enum: explicit variant URI or enum `prefix`. Prefix-generated URI changes with variant name; preserve identity via explicit URI before rename. URI stored as-is; no validation/resolution.
- `title`: string literal, default Title Case name. No interpolation.
- Static `status`: integer literal 100–999; default 500. Separate `#[problem(409)]` equivalent. No constants/expressions.
- Constants: struct `DEFINITION`; enum variant SHOUTY_SNAKE_CASE. `Problem::definitions()` lists without constructing errors.

```rust
use problems::{Problem, Report, StatusCode};

#[derive(Debug, thiserror::Error, Problem)]
#[error("record missing")]
#[problem(type_uri = "urn:example:missing-record", status = 404)]
struct Missing;

#[derive(Debug, thiserror::Error, Problem)]
#[problem(prefix = "urn:example")]
enum AppProblem {
    #[error("name conflict")]
    #[problem(409)]
    NameConflict,
    #[error(transparent)]
    #[problem(transparent)]
    Storage(#[from] Missing),
}

assert_eq!(AppProblem::NAME_CONFLICT.type_uri, "urn:example:name-conflict");
assert_eq!(AppProblem::Storage(Missing).status(), StatusCode::NOT_FOUND);
assert_eq!(AppProblem::definitions().count(), 2);

fn lookup() -> Result<(), Missing> { Err(Missing) }
fn operation() -> Result<(), AppProblem> {
    lookup()?; // Missing -> AppProblem via #[from].
    Ok(())
}
fn handler() -> Result<(), Report<AppProblem>> {
    operation()?; // AppProblem -> Report<AppProblem>.
    Ok(())
}
assert_eq!(handler().unwrap_err().problem().status(), StatusCode::NOT_FOUND);
```

Transparent: single-field tuple enum variant only. Forwards definition + runtime status + detail + instance + data. No local constant, metadata, or field selections. `#[error(transparent)]` alone forwards diagnostics only.

## Manual implementation

Without derive: supply `Data`, `DataRef<'a>`, `definition()`, `definitions()`. Optional payload/detail/instance default absent; `status()` defaults to definition status. Include every possible definition in static iterator.

```rust
use problems::{Problem, ProblemDefinition, StatusCode};

#[derive(Debug, thiserror::Error)]
#[error("record missing")]
struct Missing;

const MISSING: ProblemDefinition = ProblemDefinition {
    type_uri: "urn:example:missing-record",
    title: "Record missing",
    status: StatusCode::NOT_FOUND,
};

impl Problem for Missing {
    type Data = ();
    type DataRef<'a> = ();

    fn definition(&self) -> &'static ProblemDefinition { &MISSING }
    fn definitions() -> impl Iterator<Item = &'static ProblemDefinition> {
        std::iter::once(&MISSING)
    }
}
assert_eq!(Missing.status(), StatusCode::NOT_FOUND);
```
