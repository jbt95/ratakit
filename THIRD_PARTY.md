# Third-party and derived code

| File (in ratakit) | Origin | Commit / version | License |
|---|---|---|---|
| `crates/ratakit-text/src/highlight_syntect.rs` | `openai/codex` `codex-rs/tui/src/render/highlight.rs` | `main` (2026-10) | Apache-2.0 |
| `crates/ratakit-text/src/markdown_render.rs` | `openai/codex` `codex-rs/tui/src/markdown_render.rs`, `markdown.rs` | `main` (2026-10) | Apache-2.0 |
| `crates/ratakit-text/src/markdown_stream.rs` | `openai/codex` `codex-rs/tui/src/markdown_stream.rs` | `main` (2026-10) | Apache-2.0 |

Milestone 0 does not vendor Codex sources yet. When we port files from
[`openai/codex`](https://github.com/openai/codex) (`codex-rs/tui`), each entry
will list the exact path and commit, per the attribution plan in `docs/DISENO.md`.
