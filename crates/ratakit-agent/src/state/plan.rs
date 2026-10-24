//! Plan / task list state.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Stable id for a plan entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PlanEntryId(pub String);

impl PlanEntryId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Status of a single plan task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PlanEntryStatus {
    Pending,
    InProgress,
    Completed,
}

/// One row in an agent plan.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PlanEntry {
    pub id: PlanEntryId,
    pub title: String,
    pub status: PlanEntryStatus,
}

/// Full plan snapshot (also renderable as a transcript cell).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Plan {
    pub entries: Vec<PlanEntry>,
}
