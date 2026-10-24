//! Auxiliary layout state for tool-call cards in the transcript (widgets feature).

use ratakit_core::Motion;
use ratakit_widgets::{CodeBlockState, DiffViewState};

/// Per-transcript-row state for [`ToolCallCard`](crate::widgets::ToolCallCard) rendering.
#[derive(Debug, Default)]
pub struct ToolCallCardAux {
    pub(crate) motion: Motion,
    pub(crate) output_code: CodeBlockState,
    pub(crate) diff_view: DiffViewState,
    pub(crate) cached_revision: u64,
    pub(crate) cached_width: u16,
    pub(crate) cached_height: u16,
}
