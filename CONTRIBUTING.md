# Contributing to ratakit

Thank you for your interest in ratakit. The project is **pre-alpha**; APIs will
change until 0.1.0.

## Getting started

- Rust **1.88+** (MSRV tracks ratatui 0.30).
- `cargo test --workspace`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

Design notes live in [`docs/DISENO.md`](docs/DISENO.md) (Spanish). The English
README summarizes goals and status.

## Pull requests

- One logical change per PR when possible (see the milestone roadmap in the design doc).
- Include tests: unit tests for state in `ratakit-core` / `ratakit-agent::state`,
  insta snapshots for widgets via `ratakit-testing`.
- Run CI locally before pushing.
- For code derived from [Codex](https://github.com/openai/codex), follow the
  header and `THIRD_PARTY.md` rules in the design doc.

## License

By contributing, you agree that your contributions are licensed under the
Apache-2.0 license in `LICENSE`.
