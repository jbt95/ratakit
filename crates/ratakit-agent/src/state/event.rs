//! Inbound agent events (protocol-neutral).

use super::ids::{MessageId, ToolCallId};
use super::model::ModelChoice;
use super::plan::Plan;
use super::tool::{ToolCall, ToolCallPatch};
use super::types::{ContentPart, Level, SlashCommand, StopReason};
use super::usage::{ModeId, Usage};

/// Single entry point for transcript updates from any agent backend.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentEvent {
    UserMessage {
        id: MessageId,
        content: Vec<ContentPart>,
    },
    AssistantDelta {
        id: MessageId,
        text: String,
    },
    ThinkingDelta {
        id: MessageId,
        text: String,
    },
    MessageDone {
        id: MessageId,
    },
    ToolCallStarted(ToolCall),
    ToolCallUpdated {
        id: ToolCallId,
        patch: ToolCallPatch,
    },
    PlanUpdated(Plan),
    Usage(Usage),
    ModeChanged(ModeId),
    CommandsAvailable(Vec<SlashCommand>),
    ModelChoicesAvailable(Vec<ModelChoice>),
    Notice {
        level: Level,
        text: String,
    },
    Error(String),
    TurnEnded {
        reason: StopReason,
    },
}
