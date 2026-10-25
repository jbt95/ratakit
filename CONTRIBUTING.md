# Contributing to ratakit

Thank you for your interest in ratakit. The project is **pre-alpha**; public APIs
may change before the first stable release. Ratakit is not yet published to
crates.io.

## Getting started

- Rust **1.88+** (the workspace MSRV; ratatui 0.30 is the tested baseline).
- Start with the [User Guide](docs/USER_GUIDE.md), [Component Catalog](docs/COMPONENTS.md),
  and [Integration Cookbook](docs/COOKBOOK.md).
- Keep `ratakit-core` interaction state ratatui-free; put generic rendering in
  `ratakit-widgets`. Applications own event loops, terminal sessions, runtimes,
  key policy, focus, and side effects.

Licensing and source attribution conventions are in
[`THIRD_PARTY.md`](THIRD_PARTY.md). Do not remove or rewrite attribution notices
when editing derived code.

## Pull requests

- One logical change per PR when possible; keep changes within the agreed scope.
- Include focused tests: unit tests for state in `ratakit-core` or
  `ratakit-agent::state`, and headless render tests or snapshots for widgets via
  `ratakit-testing`.
- Update the user guide, component catalog, cookbook, and Rust API docs when a
  public behavior, feature, or integration contract changes. Keep examples
  aligned with the exported API and enabled features.
- Run the relevant checks from CI before pushing. The repository-wide checks are:

  ```sh
  cargo fmt --all -- --check
  cargo test --workspace --all-features
  cargo test --workspace --no-default-features -p ratakit-core -p ratakit-text -p ratakit-agent
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps
  cargo +1.88.0 test --workspace
  ```

- If rendered output changes, inspect the snapshot diff and run
  `cargo insta review` when snapshots need updating. The
  [CI workflow](.github/workflows/ci.yml) also runs headless examples and
  dependency/license checks.
- For code derived from [Codex](https://github.com/openai/codex), follow the
  source header and [`THIRD_PARTY.md`](THIRD_PARTY.md) rules.

## License

By contributing, you agree that your contributions are licensed under the
Apache-2.0 license in `LICENSE`.
