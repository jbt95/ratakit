//! Agent transcript, composer, and related widgets.
//!
//! The [`state`] module is ratatui-free and can be used from other UIs (for example GPUI).
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
//! ```

#![allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]

pub mod state;

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
    QueuedPayload, SlashCommand, TranscriptState,
};
