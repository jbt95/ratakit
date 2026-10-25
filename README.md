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

**Pre-alpha.** The generic widgets, agent UI, ACP bridge, accessibility profiles,
inline transcript scrollback, and user documentation are implemented. The project
is **not published to crates.io**; a 0.1.0 release is pending Jordi's approval.

Start with the [English user guide](docs/USER_GUIDE.md) and
[integration cookbook](docs/COOKBOOK.md).

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

The crates are not published yet. For a consumer app in this repository, add a
path dependency on `crates/ratakit`; the guide explains feature selection and
shows a widget-rendering example.

## Accessibility profiles

Choose `Theme::high_contrast_dark()` or `Theme::high_contrast_light()` for a
palette whose semantic text colors meet a tested 7:1 contrast ratio. For
text-first agent UI, call `theme.enable_screen_reader()`: it selects ASCII
markers and reduced motion, names transcript and tool states in text, disables
the scrollbar and side panel, and renders the composer with a text label. The
ACP examples enable the same profile with `RATAKIT_SCREEN_READER=1`.

## Examples

```bash
cargo run -p gallery
cargo run -p mock-agent
cargo run -p claude-acp   # requires Node.js/npm; launches Claude Code's ACP adapter
cargo run -p opencode-acp # requires OpenCode installed
```

The ACP examples share one `AgentLayout` app. Set `RATAKIT_INLINE=1` to use a
12-row inline viewport: each completed turn is written once to terminal
scrollback, while the current turn and composer remain live. Inline mode also
enables the text-first screen-reader profile. CI builds both launchers and runs
headless UI smoke paths without starting either real agent. To capture an
interactive ACP session, set `RATAKIT_ACP_RECORD=recording.jsonl`; recorded
event streams can be replayed and rendered as phase snapshots with:

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

## Compatibility

Versions below are the workspace's declared lower bounds and locked CI-tested
versions. They are not a claim that every future patch release has been tested.

| Integration | Declared | Locked and tested |
|---|---:|---:|
| Rust | 1.88.0 | 1.88.0 (MSRV job) |
| ratatui | 0.30.2 | 0.30.2 |
| ratatui-core | 0.1.2 | 0.1.2 |
| ratatui-widgets | 0.3.2 | 0.3.2 |
| ratatui-crossterm | 0.1.2 | 0.1.2 |
| crossterm | 0.29.0 | 0.29.0 |

## License

Apache-2.0. See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE). Codex-derived code
is tracked in [`THIRD_PARTY.md`](THIRD_PARTY.md).
