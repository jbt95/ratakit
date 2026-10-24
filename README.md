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

**Pre-alpha (milestone 4 in progress).** The generic widgets, agent UI, ACP bridge,
and Claude Code/OpenCode example clients are implemented. Recorded-session tooling
and release polish remain. **Not published to crates.io yet.**

Design (Spanish): [`docs/DISENO.md`](docs/DISENO.md).

## Workspace crates

| Crate | Role |
|---|---|
| `ratakit-core` | Events, outcomes, keymaps, focus, theme (no ratatui) |
| `ratakit-text` | Markdown streaming, highlighting, diffs |
| `ratakit-widgets` | Generic ratatui widgets |
| `ratakit-agent` | Protocol-neutral agent state + widgets |
| `ratakit-acp` | Optional ACP bridge (no ratatui dependency) |
| `ratakit-testing` | `TestBackend` snapshots, `KeySeq`, `assert_widget_snapshot!` |
| `ratakit` | Umbrella crate with feature flags |

## Quick start

```bash
cargo test --workspace
cargo run -p hello-headless
```

## Examples

```bash
cargo run -p gallery
cargo run -p mock-agent
cargo run -p claude-acp   # requires Node.js/npm; launches Claude Code's ACP adapter
cargo run -p opencode-acp # requires OpenCode installed
```

The ACP examples share one `AgentLayout` app. Set `RATAKIT_INLINE=1` to use the
inline terminal viewport. CI builds both launchers and runs their headless UI
smoke paths without starting either real agent. To capture an interactive ACP
session, set `RATAKIT_ACP_RECORD=recording.jsonl`; recorded event streams can be
replayed and rendered as phase snapshots with:

```bash
cargo run -p mock-agent -- --snapshots recording.jsonl snapshots/
```

Recording is opt-in; CI never launches a real agent, and this repository does
not bundle a live session recording.

Criterion benchmarks cover long transcript rendering, streaming Markdown, and
large diffs:

```bash
cargo bench -p ratakit-agent --bench long_transcript
cargo bench -p ratakit-text --bench text_processing --features "markdown diff highlight-syntect"
```

MSRV: **Rust 1.88** (required by ratatui 0.30).

## License

Apache-2.0. See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE). Codex-derived code
will be attributed per [`THIRD_PARTY.md`](THIRD_PARTY.md) and the design doc.
