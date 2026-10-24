//! Tool call model and patches.

use std::path::PathBuf;

use ratakit_text::DiffModel;

use super::ids::ToolCallId;

/// Lifecycle of a tool invocation in the transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// High-level tool category for icons and layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolKind {
    Read,
    Edit,
    Execute,
    Search,
    Fetch,
    Other,
}

/// Streaming or final tool output chunk.
///
/// Not serializable via the `serde` feature when it embeds [`DiffModel`].
#[derive(Debug, Clone, PartialEq)]
pub enum ToolContent {
    Text(String),
    Diff { path: PathBuf, model: DiffModel },
}

/// Tool invocation as reported by the agent.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub id: ToolCallId,
    pub title: String,
    pub kind: ToolKind,
    pub status: ToolStatus,
    pub locations: Vec<PathBuf>,
    pub content: Vec<ToolContent>,
}

/// Partial update to an in-flight or completed tool call.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ToolCallPatch {
    pub title: Option<String>,
    pub kind: Option<ToolKind>,
    pub status: Option<ToolStatus>,
    pub locations: Option<Vec<PathBuf>>,
    /// Replaces tool content when set.
    pub content: Option<Vec<ToolContent>>,
    /// Appended after any `content` replacement.
    pub content_append: Vec<ToolContent>,
    pub diff: Option<DiffModel>,
}

impl ToolCallPatch {
    /// Apply this patch onto a [`ToolCall`].
    pub fn apply_to(&self, call: &mut ToolCall) {
        if let Some(title) = &self.title {
            call.title.clone_from(title);
        }
        if let Some(kind) = self.kind {
            call.kind = kind;
        }
        if let Some(status) = self.status {
            call.status = status;
        }
        if let Some(locations) = &self.locations {
            call.locations.clone_from(locations);
        }
        if let Some(content) = &self.content {
            call.content.clone_from(content);
        }
        if !self.content_append.is_empty() {
            call.content.extend(self.content_append.clone());
        }
    }
}
