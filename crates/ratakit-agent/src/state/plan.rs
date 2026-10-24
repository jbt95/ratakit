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

impl Plan {
    /// Content revision for virtual-list height caches.
    pub fn layout_revision(&self) -> u64 {
        let mut h = self.entries.len() as u64;
        for entry in &self.entries {
            h = hash_mix(h, entry.title.len() as u64);
            h = hash_mix(h, status_tag(entry.status));
            h = hash_mix(h, entry.id.0.len() as u64);
        }
        h
    }
}

fn status_tag(status: PlanEntryStatus) -> u64 {
    match status {
        PlanEntryStatus::Pending => 0,
        PlanEntryStatus::InProgress => 1,
        PlanEntryStatus::Completed => 2,
    }
}

fn hash_mix(seed: u64, value: u64) -> u64 {
    let mut h = seed;
    h = h.wrapping_add(value);
    h = h.wrapping_add(h << 6);
    h = h.wrapping_add(h >> 2);
    h.wrapping_add(0x9e37_79b9_7f4a_7c15)
}
