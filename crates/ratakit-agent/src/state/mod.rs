//! Protocol-neutral agent transcript state (no ratatui).
//!
//! Cell layout and streaming apply logic are inspired by `openai/codex`
//! `codex-rs/tui/src/history_cell/` (Apache-2.0). See `NOTICE` and `THIRD_PARTY.md`.

mod cell;
pub mod composer;
mod event;
mod ids;
mod model;
mod permission;
mod plan;
mod tool;
mod transcript;
mod types;
mod usage;

pub use cell::{AssistantCell, Cell, ErrorCell, NoticeCell, ThinkingCell, ToolCallCell, UserCell};
pub use composer::{
    Attachment, ComposerIntent, ComposerState, FileIndex, QueuedMessage, QueuedPayload,
};
pub use event::AgentEvent;
pub use ids::{CellId, MessageId, ToolCallId};
pub use model::{Effort, ModelChoice};
pub use permission::{PermissionIntent, PermissionOption, PermissionOptionId, PermissionRequest};
pub use plan::{Plan, PlanEntry, PlanEntryId, PlanEntryStatus};
pub use tool::{ToolCall, ToolCallPatch, ToolContent, ToolKind, ToolStatus};
pub use transcript::TranscriptState;
pub use types::{ContentPart, Level, SlashCommand, StopReason};
pub use usage::{ModeId, Usage};
