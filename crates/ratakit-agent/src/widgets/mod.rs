//! Agent widgets (ratatui).

mod cells;
mod extras;
mod transcript;

pub use extras::{
    PlaceholderPlanRenderer, PlaceholderToolCallRenderer, PlanCellRenderer, ToolCallCellRenderer,
    TranscriptExtras,
};
pub use transcript::Transcript;
