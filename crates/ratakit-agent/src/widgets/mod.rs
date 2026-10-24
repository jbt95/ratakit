//! Agent widgets (ratatui).

mod cells;
mod extras;
mod transcript;

pub mod composer;
mod model_picker;
mod permission_prompt;
mod plan_view;
mod style;
mod usage_bar;

pub use composer::{Composer, ComposerModelItem, ComposerWidgetState};
pub use extras::{
    PlaceholderToolCallRenderer, PlanCellRenderer, ToolCallCellRenderer, TranscriptExtras,
};
pub use model_picker::{ModelPicker, ModelPickerIntent, ModelPickerState};
pub use permission_prompt::{PermissionPrompt, PermissionPromptModal, PermissionPromptState};
pub use plan_view::{PlanRows, PlanView, plan_cell_height, render_plan_cell};
pub use transcript::Transcript;
pub use usage_bar::{UsageBar, usage_segments};
