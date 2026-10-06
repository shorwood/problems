# Check formatting, Rust code, framework smoke tests, and dependencies.
check:
    cargo fmt --all --check
    hurlfmt --check examples/*/tests/hurl/*.hurl examples/*/tests/hurl/templates/*.hurl
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    cargo test -p problems -p problems-derive --all-features --locked
    cargo test --workspace --exclude problems --exclude problems-derive --test hurl --locked
    cargo test -p problems --no-default-features --locked
    cargo deny --all-features --locked check
