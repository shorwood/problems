# Framework examples

Each framework has its own unpublished workspace crate:

- `actix-web/` — `problems-example-actix-web`
- `axum/` — `problems-example-axum`
- `poem/` — `problems-example-poem`
- `rocket/` — `problems-example-rocket`
- `salvo/` — `problems-example-salvo`
- `warp/` — `problems-example-warp`

Each directory contains `Cargo.toml`, `src/lib.rs` for application routes,
`src/main.rs` for server startup, and `tests/hurl.rs` to run its Hurl fixtures.
This follows the structure of the
[hurl-test example](https://github.com/shorwood/hurl-test/tree/main/example).
Each crate declares its own problem type and depends on its framework.

Run one from the repository root inside `nix develop`:

```sh
cargo run -p problems-example-axum
cargo test -p problems-example-axum --test hurl --locked
```

Each executable serves on `127.0.0.1:3000`. Request `GET /problem` to see a 409
`application/problem+json` document whose public detail omits private diagnostics.
Tests bind assigned local ports and reuse the same `app()` as the executable.
All response assertions live in each crate's `tests/hurl/*.hurl`; templates are
excluded from fixture discovery.

Axum also demonstrates JSON rejection handling, response composition, and Aide
metadata. Poem demonstrates direct reports and `?` conversion. Warp demonstrates
selective application rejection recovery.

The Nix development shell provides `hurlfmt` and the native libraries needed by
`hurl-test`. From the repository root, `just check` checks fixture formatting and
runs all six crates' smoke tests alongside the library checks.
