# Contributing to ratakit

Thank you for your interest in ratakit. The project is **pre-alpha**; APIs will
change until 0.1.0.

## Getting started

- Rust **1.88+** (MSRV tracks ratatui 0.30).
- `cargo test --workspace`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

Start with the [English user guide](docs/USER_GUIDE.md) and
[integration cookbook](docs/COOKBOOK.md). Licensing and source attribution
conventions are in [`THIRD_PARTY.md`](THIRD_PARTY.md).

## Pull requests

- One logical change per PR when possible; keep changes within the agreed scope.
- Include tests: unit tests for state in `ratakit-core` / `ratakit-agent::state`,
  insta snapshots for widgets via `ratakit-testing`.
- Run CI locally before pushing.
- For code derived from [Codex](https://github.com/openai/codex), follow the
  source header and [`THIRD_PARTY.md`](THIRD_PARTY.md) rules.

## License

By contributing, you agree that your contributions are licensed under the
Apache-2.0 license in `LICENSE`.
