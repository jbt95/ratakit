//! Permission prompts (ACP-shaped, but protocol-neutral).

use ratakit_text::DiffModel;

use super::tool::{ToolCall, ToolContent};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// User's choice when responding to a permission prompt.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PermissionIntent {
    Chose(PermissionOptionId),
    Dismissed,
}

/// Identifier for a permission option (maps to ACP outcomes in the adapter).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct PermissionOptionId(pub String);

impl PermissionOptionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// One selectable permission outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PermissionOption {
    pub id: PermissionOptionId,
    pub label: String,
    /// When true, the choice applies to similar future requests.
    pub persistent: bool,
}

/// Tool call the user is asked to approve or deny.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PermissionRequest {
    pub tool_call: ToolCall,
    pub options: Vec<PermissionOption>,
}

/// Body content shown above permission action buttons.
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionPreview {
    /// Shell command or similar text payload.
    Command { text: String },
    /// File change the user is approving.
    Diff(DiffModel),
}

impl PermissionRequest {
    /// Picks the most relevant preview for the permission UI (diff preferred over text).
    pub fn preview(&self) -> PermissionPreview {
        for chunk in &self.tool_call.content {
            if let ToolContent::Diff { model, .. } = chunk {
                return PermissionPreview::Diff(model.clone());
            }
        }
        let lines: Vec<&str> = self
            .tool_call
            .content
            .iter()
            .filter_map(|chunk| match chunk {
                ToolContent::Text(text) => Some(text.as_str()),
                ToolContent::Diff { .. } => None,
            })
            .collect();
        if !lines.is_empty() {
            return PermissionPreview::Command {
                text: lines.join("\n"),
            };
        }
        PermissionPreview::Command {
            text: self.tool_call.title.clone(),
        }
    }
}
