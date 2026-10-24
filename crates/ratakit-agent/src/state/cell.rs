//! Transcript cells.

use ratakit_text::{DiffModel, MarkdownOptions, MarkdownStream};

use super::ids::{MessageId, ToolCallId};
use super::plan::Plan;
use super::tool::{ToolCall, ToolKind, ToolStatus};
use super::types::{ContentPart, Level};

/// One row in the transcript.
#[derive(Debug)]
pub enum Cell {
    User(UserCell),
    Assistant(AssistantCell),
    Thinking(ThinkingCell),
    ToolCall(ToolCallCell),
    Plan(Plan),
    Notice(NoticeCell),
    Error(ErrorCell),
}

/// User-authored message.
#[derive(Debug)]
pub struct UserCell {
    pub id: MessageId,
    pub content: Vec<ContentPart>,
}

/// Assistant reply with incremental markdown rendering.
#[derive(Debug)]
pub struct AssistantCell {
    pub id: MessageId,
    pub stream: MarkdownStream,
    pub done: bool,
}

impl AssistantCell {
    pub(crate) fn new(id: MessageId) -> Self {
        Self {
            id,
            stream: MarkdownStream::new(MarkdownOptions::default()),
            done: false,
        }
    }
}

/// Agent reasoning / thought stream (folded by default when complete).
#[derive(Debug)]
pub struct ThinkingCell {
    pub id: MessageId,
    pub stream: MarkdownStream,
    pub folded: bool,
    pub done: bool,
}

impl ThinkingCell {
    pub(crate) fn new(id: MessageId) -> Self {
        Self {
            id,
            stream: MarkdownStream::new(MarkdownOptions::default()),
            folded: false,
            done: false,
        }
    }
}

/// Tool invocation card state.
#[derive(Debug)]
pub struct ToolCallCell {
    pub call: ToolCall,
    pub input: String,
    pub output: String,
    pub diff: Option<DiffModel>,
    pub output_folded: bool,
}

impl ToolCallCell {
    pub(crate) fn from_call(call: ToolCall) -> Self {
        Self {
            call,
            input: String::new(),
            output: String::new(),
            diff: None,
            output_folded: false,
        }
    }

    pub fn id(&self) -> &ToolCallId {
        &self.call.id
    }

    pub fn status(&self) -> ToolStatus {
        self.call.status
    }

    pub fn kind(&self) -> ToolKind {
        self.call.kind
    }
}

/// Informational notice (compaction, mode hints, …).
#[derive(Debug, Clone)]
pub struct NoticeCell {
    pub level: Level,
    pub text: String,
}

/// Fatal or turn-level error message.
#[derive(Debug, Clone)]
pub struct ErrorCell {
    pub text: String,
}
