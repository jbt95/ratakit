//! Agent widgets (ratatui).

mod cells;
mod extras;
mod transcript;

pub mod composer;
mod model_picker;
mod permission_prompt;

pub use composer::{Composer, ComposerModelItem, ComposerWidgetState};
pub use extras::{
    PlaceholderPlanRenderer, PlaceholderToolCallRenderer, PlanCellRenderer, ToolCallCellRenderer,
    TranscriptExtras,
};
pub use model_picker::{ModelPicker, ModelPickerIntent, ModelPickerState};
pub use permission_prompt::{PermissionPrompt, PermissionPromptModal, PermissionPromptState};
pub use transcript::Transcript;
