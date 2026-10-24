//! Model and reasoning effort selection.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Reasoning effort level for a model (when exposed by the agent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Effort {
    Low,
    Medium,
    High,
    ExtraHigh,
}

/// A selectable model entry for the model picker.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ModelChoice {
    pub provider: String,
    pub model: String,
    pub effort: Option<Effort>,
}
