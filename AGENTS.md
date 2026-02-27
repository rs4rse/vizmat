# Agent Notes

- Before pushing, run `cargo fmt --all -- --check`.
- If the check fails, run `cargo fmt --all`, then re-run `cargo fmt --all -- --check` until it passes.
- Before pushing, run `cargo clippy -- -D warnings`.
- If clippy fails, fix warnings/errors and re-run `cargo clippy -- -D warnings` until it passes.
