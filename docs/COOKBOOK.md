# ratakit Integration Cookbook

ratakit provides state and widgets; the application owns its event loop, terminal session, runtime,
agent process, and key bindings. The complete working integration is in
[`examples/acp-app/src/lib.rs`](../examples/acp-app/src/lib.rs). This page explains the seams to
keep when adapting it.

## Drive an agent layout

Keep the `Theme`, `AgentLayoutState`, file index, and keymap in application state. Feed inbound
agent events into `AgentLayoutState::apply_agent_event`, then render the layout each frame. For
input, handle application-level shortcuts first and pass the remaining events to
`AgentLayoutState::handle`:

```rust,ignore
if is_quit_shortcut(&event) {
    app.should_quit = true;
} else if let Outcome::Intent(intent) =
    app.layout.handle(&event, &app.keymap, &app.files)
{
    app.handle_intent(intent, &agent_handle)?;
}
```

Translate intents at the application seam: submit text to the agent, forward cancellation, return
permission choices using the request ID held by the application, and apply model/mode changes.
Keep potentially slow agent operations asynchronous so input and rendering continue. On each
terminal resize, resize the terminal and schedule a redraw. Keep the guard returned by
`enter_terminal_with` alive for the session so terminal modes are restored when the app exits.

For ACP, `AcpBridge` supplies commands and inbound events; it does not own the UI loop. The shared
example also demonstrates how to handle permission IDs, background operations, inline scrollback,
and terminal cleanup.

## Keep shortcuts and help text aligned

Define actions in the app's `Keymap`, then derive display hints from that same map. Labels are
application-owned, so ratakit does not impose names or keys:

```rust,ignore
let mut labels = ActionLabels::new();
labels.insert("composer.submit", "send");
let hints = key_hints_from_keymap(&keymap, &labels, 5);
KeyHints::new(&theme, &hints).render(area, buffer);
```

Recompute hints when the keymap revision changes. `key_hints_revision(&keymap)` is available for
cache keys.

## Navigate transcript search results

Search returns ranked results with bounded previews. Keep the navigator in the application to wire
those results to its own prompt and keys; cycling wraps at either end, and a transcript mutation
invalidates the navigator. Jumping selects the cell and underlines the matched graphemes on the
next transcript render:

```rust,ignore
let mut search = transcript.search_navigator("permission timeout", 50);
if transcript.navigate_search(&mut search, SearchDirection::Next) {
    let current = search.selected().expect("a result was selected");
    show_result_preview(&current.preview);
}
```

## Record and replay safely

Enable the `recording` feature to use `RecordingWriter` and `RecordingReader`. Choose the path and
retention policy in the application; recording is opt-in and can contain prompts, tool output, and
file paths. Use the redaction callbacks before writing sensitive values:

```rust,ignore
let mut recording = RecordingWriter::new(writer)?;
recording.event_with_redaction(&event, |event| {
    if let AgentEvent::UserMessage { content, .. } = event {
        redact_user_content(content);
    }
})?;
```

The versioned JSONL reader also accepts the earlier example format. `mock-agent` demonstrates
turn-separated replay and rendering snapshots from a recording.

## Test interactions and accessible output

Use `ComponentHarness` with `KeySeq` to send deterministic input and inspect `Outcome`s and
component state. Use `render_stateful_buffer` when a test needs to inspect styles or terminal
cells, and `assert_accessible_labels` for screen-reader copy. For full layout tests, render with a
`TestBackend` at narrow, standard, and wide sizes; include high-contrast and screen-reader themes.
For custom RGB palettes, inspect `Palette::text_contrast_ratios()` or use
`assert_palette_text_contrast(&palette, 4.5)` in tests. Indexed terminal colors are reported as
unmeasurable rather than assigned a guessed RGB value.

Measure repeated search before introducing a cache:

```sh
cargo bench -p ratakit-agent --bench long_transcript -- transcript_search --sample-size 10
```

The library intentionally does not provide a universal application runner: applications have
different runtimes, terminal backends, and agent lifecycles. Compose these helpers at the seams
that vary in your app.

## Check a release without publishing

Run the read-only preflight after setting the intended release versions:

```sh
python3 scripts/release_preflight.py --version 0.1.0
cargo deny check licenses
```

The script checks publishable workspace packages, package contents, version alignment, metadata,
and local dependency order. It never reserves crate names or publishes. A successful local check
does not replace the required maintainer approval or the CI release review.
