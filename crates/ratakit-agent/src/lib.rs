//! Agent transcript, composer, and related widgets.
//!
//! The [`state`] module is ratatui-free and can be used from other UIs (for example GPUI).

#![allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]

pub mod state;

#[cfg(feature = "widgets")]
pub mod widgets;

#[cfg(feature = "widgets")]
pub use widgets::{
    Composer, ComposerModelItem, ComposerWidgetState, ModelPicker, ModelPickerIntent,
    ModelPickerState, PermissionPrompt, PermissionPromptModal, PermissionPromptState, PlanRows,
    PlanView, UsageBar, plan_cell_height, render_plan_cell, usage_segments,
};

pub use state::{
    AgentEvent, Attachment, Cell, ComposerIntent, ComposerState, FileIndex, QueuedMessage,
    QueuedPayload, SlashCommand, TranscriptState,
};
