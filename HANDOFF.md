# ratakit Handoff

Project status as of October 25, 2026, and the remaining work toward 0.1.0. The full design and roadmap are in [`docs/DISENO.md`](docs/DISENO.md); the agent module API is in §7.4 and the roadmap is in §14.

## Current status

Milestones 0–3 are complete. C1–C3 and R1 are implemented on the current branch;
the ACP example build, headless smoke paths, recorder round trip, replay snapshot
conversion, and accessibility checks pass locally. No real-agent recording is
bundled; live recording is opt-in and no live agent was launched for this work.

| Milestone | Scope | PRs |
|---|---|---|
| 0 | Workspace, CI, `ratakit-core`, `ratakit-testing` | #1 |
| 1 | G1–G6: `TextArea`, `VirtualList`, `Highlighter`/`CodeBlock`, `Spinner`/`StatusBar`/`KeyHints`/`Toasts`, `Modal`/`Tabs`/`Table`, `FuzzyFilter` | #2–#7 |
| 2 | G7–G12: `Markdown`, `DiffView`, `FuzzyPicker`, `CompletionMenu`, `Tree`/`ScrollView`, `FrameScheduler`/`event_stream`/`enter_terminal` + `gallery` example | #8–#13 |
| 3 | A1–A8: `agent::state`, `Transcript`, `ToolCallCard`, `Composer`, `PermissionPrompt`, `ModelPicker`, `UsageBar`/`PlanView`, `AgentLayout` + `mock-agent` example | #14–#21 |

`ratakit-acp` now contains the ACP bridge implemented for C1. The remaining milestone-4 and milestone-5 tasks are listed below.

### Running the examples

```sh
cargo run -p gallery       # Generic components, organized into tabs
cargo run -p mock-agent    # The agent layout replaying .jsonl scripts
cargo run -p claude-acp    # Requires Node.js/npm; starts Claude Code's ACP adapter
cargo run -p opencode-acp  # Requires OpenCode installed
RATAKIT_SCREEN_READER=1 cargo run -p claude-acp # Text-first accessible UI
RATAKIT_INLINE=1 cargo run -p mock-agent   # Opt-in inline mode
RATAKIT_INLINE=1 cargo run -p opencode-acp # Opt-in inline mode for an ACP example
RATAKIT_ACP_RECORD=recording.jsonl cargo run -p claude-acp # Record a real ACP session
cargo run -p mock-agent -- --snapshots recording.jsonl snapshots/ # Render replay snapshots
cargo bench -p ratakit-agent --bench long_transcript
cargo bench -p ratakit-text --bench text_processing --features "markdown diff highlight-syntect"
```

## Remaining tasks

Each row represents one PR. Independent work may proceed in parallel.

### Milestone 4: ACP and real-agent examples

- [x] **C1. `ratakit-acp`: `AcpBridge`.** Depends on A1 and A5 (complete).
  - Map ACP `SessionUpdate` values to `AgentEvent` values (messages, thinking, tool calls and diffs, plans, and usage).
  - Forward permission requests to the UI and send the user's decision back to the agent.
  - Send prompts, cancellations, and model or mode changes.
  - Test against a fake ACP agent over an in-memory transport, without network access.
  - Keep the crate free of ratatui dependencies so a review client such as revu can reuse it.
- [x] **C2. `examples/claude-acp` and `examples/opencode-acp`.** Depends on C1 and A8.
  - Build a shared app of about 100 lines that launches an agent over stdio and renders it with `AgentLayout`; each example changes only the launch command.
  - CI must compile the examples but must not launch the real agents.
- [x] **C3. Recorded scripts and benchmarks.** Depends on C2.
  - Opt-in real sessions record as `.jsonl`; convert them to snapshots and replay them in `mock-agent`. A real recording is not bundled because no live agent was launched.
  - Add Criterion benchmarks for long transcript rendering, streaming Markdown, and large diffs.

### Milestone 5: polish and 0.1

- [x] **R1. Accessibility.** Screen-reader profile, high-contrast palettes, and a `MotionMode` audit (static indicators when reduced or screen-reader mode is enabled). Depends on A8. Transcript scrollback integration is completed by R2.
- [ ] **R2. Complete inline mode.** Send transcript history to terminal scrollback (inspired by Codex's `insert_history.rs`). Inline mode is currently opt-in, but the transcript is not yet written to scrollback. Depends on A2.
- [ ] **R3. Documentation.** User guide, docs.rs examples, README, and a ratatui compatibility table. Depends on all preceding work.
- [ ] **R4. crates.io 0.1.0 release.** Reserve crate names and publish. **Requires Jordi's approval.** Depends on R1–R3.

## Decisions

- Alternate-screen mode is the default; inline mode is opt-in.
- Syntect is the only syntax highlighter in v1; tree-sitter is out of scope.
- “Fullscreen” refers only to the terminal viewport (`TerminalViewport`, `enter_terminal()`). Widget enums must not have a `Fullscreen` variant; use names such as `Fill` or `SingleLine` instead.
- The ACP bridge does not advertise filesystem or terminal capabilities. Revisit this only after 0.1.
- New and revised project text should be written in English.

## Open questions

1. Should ratakit be integrated with the review client revu? `ratakit-agent` without the `widgets` feature and `ratakit-acp` do not depend on ratatui and could be shared. If needed, `agent::state` could move into its own `ratakit-agent-state` crate.

## Conventions and lessons

- **CI (10 checks):** `fmt`, `clippy` (with `-D warnings`), `test`, `msrv` (1.88), `deny`, `docs`, `hello-headless`, `gallery-headless`, `mock-agent-headless`, and `acp-examples`. Do not rename existing checks; new jobs are fine.
- **Merge:** squash-merge when CI is green and the branch is up to date with `main`.
- **Parallel PRs:** changes often overlap in `widgets/mod.rs`, `lib.rs`, `state/mod.rs`, and `Cargo.toml`. After each merge, branches must merge `main` and preserve existing modules and exports. Resolve conflicts by merging `main`, not by requesting approval.
- **Composable modules:** each component should expose render functions or traits from its own module (for example, `render_tool_call_card` and `render_plan_cell`) so other work can use it without editing its files.
- **Snapshots:** keep them compact (about 40 lines for small terminals). Do not add snapshots thousands of lines long.
- **Headless mode:** each example has a variable such as `RATAKIT_GALLERY_HEADLESS=1` or `RATAKIT_MOCK_HEADLESS=1` that renders a few frames in a `TestBackend` and exits with status 0. New examples should follow this pattern.
- **Layers:** `ratakit-core` and `ratakit-agent::state` do not depend on ratatui; agent widgets are gated behind the `widgets` feature.
