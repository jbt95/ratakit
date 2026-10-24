//! Shared value types for agent events.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Severity for [`super::AgentEvent::Notice`] and notice cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Level {
    Info,
    Warning,
    Error,
}

/// A fragment of a user message (text or attachment placeholder).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ContentPart {
    Text(String),
    /// Image bytes are held by the application; the transcript only shows a marker.
    Image {
        alt: String,
    },
}

/// Slash command exposed by the agent for the composer menu.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub argument_hint: Option<String>,
}

/// Why an agent turn stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    Cancelled,
    Refusal,
    Error,
}
