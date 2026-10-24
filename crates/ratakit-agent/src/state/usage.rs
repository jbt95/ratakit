//! Usage meters and session mode.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Agent-reported token and cost usage for the status bar.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub context_used: Option<f32>,
    pub cost_usd: Option<f64>,
}

/// Session mode identifier (`plan`, `auto`, `ask`, …).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ModeId(pub String);

impl ModeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
