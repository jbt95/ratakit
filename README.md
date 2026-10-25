# ratakit

**ratakit** is an open-source (Apache-2.0) Rust TUI component library built on
[ratatui](https://github.com/ratatui/ratatui). It has two layers:

1. **Generic widgets** — buttons, form controls, visual primitives, editors,
   lists, Markdown, diffs, pickers, and other reusable pieces.
2. **Agentic coding UI** — transcript, composer, tool cards, permission prompts,
   model picker, usage bar, and an optional [ACP](https://agentclientprotocol.com/)
   adapter for clients like Claude Code or opencode.

The library follows ratatui’s `Widget` / `StatefulWidget` pattern. Interaction
state is separated from rendering so it can be tested without a terminal;
widgets are thin, per-frame renderers and the application owns the event loop.

## Status

**Pre-alpha.** The public API may change before the first stable release. Ratakit
is **not published to crates.io**; use the workspace and path-dependency examples
below while evaluating it.

## Start here

- **Build an application:** start with the [User Guide](docs/USER_GUIDE.md) for
  crate and feature selection, then use the [Integration Cookbook](docs/COOKBOOK.md)
  for event handling and app ownership boundaries.
- **Choose a widget:** browse the [Component Catalog](docs/COMPONENTS.md), then
  run `cargo run -p gallery` for an interactive showcase.
- **Contribute:** read [CONTRIBUTING.md](CONTRIBUTING.md) before changing the
  workspace. LLM guidance is in [`skills/ratakit/SKILL.md`](skills/ratakit/SKILL.md).

## Workspace crates

| Crate | Role |
|---|---|
| `ratakit-core` | Events, outcomes, keymaps, focus, theme (no ratatui) |
| `ratakit-text` | Styled text, wrapping, optional Markdown, highlighting, and diffs |
| `ratakit-widgets` | Generic ratatui widgets |
| `ratakit-agent` | Protocol-neutral agent state + widgets |
| `ratakit-acp` | Optional ACP bridge (no ratatui dependency) |
| `ratakit-testing` | `TestBackend` snapshots, `KeySeq`, `assert_widget_snapshot!` |
| `ratakit` | Umbrella crate with feature flags |

## Check out and run

```bash
cargo +1.88.0 test --workspace --all-features
cargo run -p hello-headless
cargo run -p gallery
```

For an application in this checkout, add a path dependency on `crates/ratakit`.
The [User Guide](docs/USER_GUIDE.md#add-the-umbrella-crate) shows the dependency,
feature selection, and a headless widget-rendering example. The umbrella crate
keeps optional integrations behind Cargo features; enable only what the app uses.

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
