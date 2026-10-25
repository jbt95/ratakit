## Summary

<!-- What does this PR change? -->

## Contract and scope

<!-- What should callers observe? Which crate or integration boundary changes? -->

- [ ] Public behavior and feature requirements are clear.
- [ ] User docs, component catalog, cookbook, and Rust API docs are updated as needed.
- [ ] Source attribution is preserved for derived code.

## Testing

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --workspace --all-features`
- [ ] `cargo test --workspace --no-default-features -p ratakit-core -p ratakit-text -p ratakit-agent` (if feature behavior changes)
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps`
- [ ] `cargo +1.88.0 test --workspace`
- [ ] Render snapshots inspected/updated if UI changed
- [ ] Headless gallery or app smoke run if the UI flow changed

<!-- Note skipped checks and why; include additional focused commands below. -->

See [`workflows/ci.yml`](workflows/ci.yml) for the full CI matrix.
