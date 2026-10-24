//! Agent widgets built on [`ratakit_widgets`].

mod agent_layout;
mod cells;
mod extras;
mod transcript;

pub mod composer;

mod model_picker;
mod permission_prompt;
mod plan_view;
mod style;
mod tool_call_card;
mod usage_bar;

pub use agent_layout::{
    AgentLayout, AgentLayoutIntent, AgentLayoutState, focus as agent_layout_focus,
};
pub use composer::{Composer, ComposerModelItem, ComposerWidgetState};
pub use model_picker::{ModelPicker, ModelPickerIntent, ModelPickerState};
pub use permission_prompt::{PermissionPrompt, PermissionPromptModal, PermissionPromptState};
pub use plan_view::{PlanRows, PlanView, plan_cell_height, render_plan_cell};
pub use tool_call_card::{
    ToolCallCard, ToolCallCardAux, ToolCallTranscriptRenderer, measure_tool_call_card,
    render_tool_call_card,
};
pub use transcript::{Transcript, TranscriptHistory};
pub use usage_bar::{UsageBar, usage_segments, usage_segments_compact};
