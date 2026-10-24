//! Tool call model and patches.

use std::path::PathBuf;

use ratakit_text::DiffModel;

use super::ids::ToolCallId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Lifecycle of a tool invocation in the transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ToolStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// High-level tool category for icons and layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ToolCallPatch {
    pub title: Option<String>,
    pub kind: Option<ToolKind>,
    pub status: Option<ToolStatus>,
    pub locations: Option<Vec<PathBuf>>,
    /// Replaces tool content when set.
    pub content: Option<Vec<ToolContent>>,
    /// Appended after any `content` replacement.
    pub content_append: Vec<ToolContent>,
    #[cfg_attr(feature = "serde", serde(skip))]
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

#[cfg(feature = "serde")]
mod tool_content_serde {
    use super::{DiffModel, ToolContent};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::path::PathBuf;

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum ToolContentWire {
        Text { text: String },
        Diff {
            path: PathBuf,
            old_text: String,
            new_text: String,
        },
    }

    fn to_wire(content: &ToolContent) -> ToolContentWire {
        match content {
            ToolContent::Text(text) => ToolContentWire::Text {
                text: text.clone(),
            },
            ToolContent::Diff { path, model: _ } => ToolContentWire::Diff {
                path: path.clone(),
                old_text: String::new(),
                new_text: String::new(),
            },
        }
    }

    fn from_wire(wire: ToolContentWire) -> ToolContent {
        match wire {
            ToolContentWire::Text { text } => ToolContent::Text(text),
            ToolContentWire::Diff {
                path,
                old_text,
                new_text,
            } => {
                let path_label = path.to_string_lossy().into_owned();
                ToolContent::Diff {
                    path,
                    model: DiffModel::from_texts(&path_label, &old_text, &new_text),
                }
            }
        }
    }

    impl Serialize for ToolContent {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            to_wire(self).serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for ToolContent {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            Ok(from_wire(ToolContentWire::deserialize(deserializer)?))
        }
    }
}
