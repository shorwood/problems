# Check formatting, Rust code, workspace features, and dependencies.
check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    cargo test --workspace --all-features --locked
    cargo test -p problems --no-default-features --locked
    cargo deny --all-features --locked check
