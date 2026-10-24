//! Permission prompts (ACP-shaped, but protocol-neutral).

use super::tool::ToolCall;

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
pub struct PermissionRequest {
    pub tool_call: ToolCall,
    pub options: Vec<PermissionOption>,
}
