# ratakit

**ratakit** is an open-source (Apache-2.0) Rust TUI component library built on
[ratatui](https://github.com/ratatui/ratatui). It has two layers:

1. **Generic widgets** — editors, lists, markdown, diffs, pickers, and other
   pieces useful in any terminal UI.
2. **Agentic coding UI** — transcript, composer, tool cards, permission prompts,
   model picker, usage bar, and an optional [ACP](https://agentclientprotocol.com/)
   adapter for clients like Claude Code or opencode.

The library follows ratatui’s `Widget` / `StatefulWidget` pattern: **state and
logic live in crates without a ratatui dependency** so you can unit-test without
a terminal; widgets are thin renderers per frame.

## Status

**Pre-alpha (milestone 0).** The workspace layout, `ratakit-core`, `ratakit-testing`,
CI, and docs are in place. Widgets and the agent module are stubs until later
milestones. **Not published to crates.io yet.**

Design (Spanish): [`docs/DISENO.md`](docs/DISENO.md).

## Workspace crates

| Crate | Role |
|---|---|
| `ratakit-core` | Events, outcomes, keymaps, focus, theme (no ratatui) |
| `ratakit-text` | Markdown streaming, highlighting, diffs (stub) |
| `ratakit-widgets` | Generic ratatui widgets (stub) |
| `ratakit-agent` | Agent state + widgets (stub) |
| `ratakit-acp` | Optional ACP bridge (stub) |
| `ratakit-testing` | `TestBackend` snapshots, `KeySeq`, `assert_widget_snapshot!` |
| `ratakit` | Umbrella crate with feature flags |

## Quick start

```bash
cargo test --workspace
cargo run -p hello-headless
```

MSRV: **Rust 1.88** (required by ratatui 0.30).

## License

Apache-2.0. See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE). Codex-derived code
will be attributed per [`THIRD_PARTY.md`](THIRD_PARTY.md) and the design doc.
