//! Protocol-neutral agent transcript state and optional Ratatui widgets.
//!
//! The [`state`] module has no ratatui dependency and can be used from other UIs
//! (for example GPUI). It stores protocol-neutral events and state; applications
//! own agent processes, event loops, terminal setup, and side effects.
//!
//! The crate's `widgets` feature adds the composer, transcript, permission,
//! model-picker, plan, and usage views. Disable default features when only the
//! protocol-neutral state is needed.
//!
//! # Apply protocol-neutral events
//!
//! ```
//! use ratakit_agent::state::{AgentEvent, MessageId, StopReason, TranscriptState};
//!
//! let mut transcript = TranscriptState::new();
//! let id = MessageId::new("assistant-1");
//! transcript.apply(AgentEvent::AssistantDelta {
//!     id: id.clone(),
//!     text: "Hello from the agent.".into(),
//! });
//! transcript.apply(AgentEvent::MessageDone { id });
//! transcript.apply(AgentEvent::TurnEnded {
//!     reason: StopReason::EndTurn,
//! });
//! assert_eq!(transcript.cells().len(), 1);
//! let matches = transcript.search("Hello", 5);
//! assert_eq!(matches[0].cell_index, 0);
//! ```

#![allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]

pub mod state;

#[cfg(feature = "recording")]
pub mod recording;

#[cfg(feature = "widgets")]
mod tool_call_aux;

#[cfg(feature = "widgets")]
pub mod widgets;

#[cfg(feature = "widgets")]
pub use widgets::{
    AgentLayout, AgentLayoutIntent, AgentLayoutState, Composer, ComposerModelItem,
    ComposerWidgetState, ModelPicker, ModelPickerIntent, ModelPickerState, PermissionPrompt,
    PermissionPromptModal, PermissionPromptState, PlanRows, PlanView, ToolCallCard,
    ToolCallCardAux, ToolCallTranscriptRenderer, Transcript, TranscriptHistory, UsageBar,
    agent_layout_focus, measure_tool_call_card, plan_cell_height, render_plan_cell,
    render_tool_call_card, usage_segments, usage_segments_compact,
};

pub use state::{
    AgentEvent, Attachment, Cell, ComposerIntent, ComposerState, FileIndex, QueuedMessage,
    QueuedPayload, SearchDirection, SlashCommand, TranscriptSearchNavigator,
    TranscriptSearchResult, TranscriptState,
};
