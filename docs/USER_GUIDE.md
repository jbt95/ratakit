# ratakit User Guide

ratakit is a Rust terminal UI component library built on ratatui. It provides
generic widgets, a protocol-neutral agent UI, and an optional Agent Client
Protocol (ACP) bridge. The application still owns its event loop, runtime,
terminal backend, and agent process.

> **Release status:** ratakit is not published on crates.io yet. A crates.io
> dependency example will be added after the 0.1.0 release is approved. The
> snippets below assume a checkout of this repository.

## Choose a crate

| Crate | Use it for |
|---|---|
| `ratakit` | The umbrella API and opt-in feature selection |
| `ratakit-core` | Events, keymaps, focus, themes, and state without ratatui |
| `ratakit-text` | Styled text, streaming Markdown, highlighting, and diffs |
| `ratakit-widgets` | Generic ratatui widgets |
| `ratakit-agent` | Protocol-neutral agent state and optional agent widgets |
| `ratakit-acp` | Connecting an ACP agent to `ratakit-agent::state` |
| `ratakit-testing` | TestBackend rendering and snapshot helpers |

The core state crates do not depend on ratatui. Agent widgets are enabled by the
`widgets` feature of `ratakit-agent`; the ACP bridge itself does not depend on
ratatui.

## Add the umbrella crate

For a consumer package inside this checkout, point at the umbrella crate. Add
ratatui and a backend crate to the application as needed; ratakit widget crates
depend on `ratatui-core`, not on the ratatui facade.

```toml
[dependencies]
ratakit = { path = "../ratakit/crates/ratakit" }
ratatui = "0.30.2"
ratatui-crossterm = "0.1.2"
```

The umbrella crate's defaults enable `widgets`, `markdown`, `highlight-syntect`,
`diff`, `fuzzy`, and `crossterm`. Disable defaults to choose a smaller set:

```toml
[dependencies]
ratakit = {
  path = "../ratakit/crates/ratakit",
  default-features = false,
  features = ["widgets", "markdown", "crossterm"],
}
```

Useful optional umbrella features:

| Feature | Enables |
|---|---|
| `widgets` | Generic widget types |
| `markdown` | Markdown widget and parser |
| `highlight-syntect` | Syntect syntax highlighting |
| `diff` | Diff model and widget |
| `fuzzy` | Fuzzy picker and completion support |
| `crossterm` | Crossterm event conversion and terminal session helpers |
| `tokio` | Async event stream and frame scheduler |
| `agent` | Protocol-neutral agent state and widgets |
| `acp` | Agent feature, ACP bridge, and Tokio integration |
| `serde` | Serialization support for the types that expose it |

`highlight-syntect-onig` uses Syntect's Oniguruma backend and may require a C
toolchain. The default Syntect backend uses the pure-Rust `fancy-regex` feature.

## Render a widget

Keep the widget state in your application and render it for each frame. The
following example uses `TestBackend` so it needs no interactive terminal:

```rust
use ratakit::core::Theme;
use ratakit::widgets::{TextArea, TextAreaState};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

let theme = Theme::dark();
let mut input = TextAreaState::new();
let mut terminal = Terminal::new(TestBackend::new(40, 6))?;

terminal.draw(|frame| {
    frame.render_stateful_widget(
        TextArea::new(&theme).placeholder("Write a message…"),
        frame.area(),
        &mut input,
    );
})?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

In an application, route normalized `ratakit::core::Event` values to the state
through its `Component::handle` method. Handle the returned `Outcome` and
perform application side effects—such as submitting text—in the event loop.
ratakit does not own that loop.

## Build an agent UI

Enable `agent` on the umbrella crate. Apply protocol-neutral events to
`TranscriptState`; it retains the transcript and agent metadata independently
of rendering:

```rust
use ratakit::agent::state::{AgentEvent, MessageId, StopReason, TranscriptState};

let mut transcript = TranscriptState::new();
let id = MessageId::new("assistant-1");
transcript.apply(AgentEvent::AssistantDelta {
    id: id.clone(),
    text: "Hello from the agent.".into(),
});
transcript.apply(AgentEvent::MessageDone { id });
transcript.apply(AgentEvent::TurnEnded {
    reason: StopReason::EndTurn,
});
assert_eq!(transcript.cells().len(), 1);
```

With the default `widgets` feature, `AgentLayout` combines the transcript,
composer, permissions, model picker, usage, and plan widgets. The
`examples/mock-agent` application demonstrates a deterministic event loop and
replay scripts. `examples/acp-app` demonstrates connecting that layout to an
ACP bridge.

## Terminal and inline modes

`TerminalViewport::AlternateScreen` is the default for `enter_terminal()`.
Inline mode uses the main terminal buffer, so create the Ratatui terminal with a
matching inline viewport as well:

```rust
use ratakit::widgets::TerminalViewport;
use ratatui::TerminalOptions;

let options = TerminalOptions {
    viewport: TerminalViewport::Inline.ratatui_viewport(12),
};
```

The app examples provide the complete setup. Set `RATAKIT_INLINE=1` for their
12-row viewport. Each completed agent turn is inserted once into terminal
scrollback; the active turn and composer remain in the viewport. Inline mode
also enables the text-first screen-reader profile. For custom apps, use
`TranscriptState::take_completed_cells` and `TranscriptHistory` to manage that
archive yourself.

## Accessibility

- Use `Theme::high_contrast_dark()` or `Theme::high_contrast_light()` for the
  tested high-contrast palettes.
- Call `Theme::enable_screen_reader()` for text labels, ASCII markers, reduced
  motion, and a simpler agent layout.
- Keep all functionality available through your key bindings; ratakit does not
  provide application-specific focus or command policy.
- The ACP examples enable the screen-reader profile with
  `RATAKIT_SCREEN_READER=1`.

## Tests and examples

Run the workspace checks from the repository root:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps
```

`ratakit-testing` includes `render_widget`, `render_stateful`, and snapshot
helpers for headless widget tests. The examples can be run with:

```sh
cargo run -p gallery
cargo run -p mock-agent
cargo run -p claude-acp   # requires Node.js/npm and Claude Code authentication
cargo run -p opencode-acp # requires OpenCode
```

The ACP examples never launch real agents in CI. Recording a live interactive
session is opt-in with `RATAKIT_ACP_RECORD=recording.jsonl`; this repository does
not bundle a live recording.

## Compatibility

The current tested dependency baseline and compatibility caveats are listed in
the [README compatibility table](../README.md#compatibility). ratakit targets
Rust 1.88 and the ratatui 0.30 split-crate API. Backends other than Crossterm
have not been validated by this workspace.
