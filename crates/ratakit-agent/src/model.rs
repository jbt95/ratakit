//! Model catalog types for agent configuration pickers.
//!
//! A1 (`agent::state`) should re-export or alias these types instead of duplicating them.

use std::fmt;

/// Reasoning effort level exposed by an agent backend.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effort {
    Low,
    Medium,
    High,
    /// Provider-specific labels (for example `xhigh`).
    Other(String),
}

impl Effort {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Other(label) => label,
        }
    }

    pub fn parse(label: &str) -> Option<Self> {
        match label {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            other if !other.is_empty() => Some(Self::Other(other.to_string())),
            _ => None,
        }
    }

    /// Stable ordering for secondary-axis chips (known levels first, then lexicographic).
    pub fn sort_key(&self) -> (u8, String) {
        match self {
            Self::Low => (0, String::new()),
            Self::Medium => (1, String::new()),
            Self::High => (2, String::new()),
            Self::Other(label) => (3, label.clone()),
        }
    }
}

impl PartialOrd for Effort {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Effort {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl fmt::Display for Effort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Active model configuration (provider + model id + optional reasoning effort).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelChoice {
    pub provider: String,
    pub model: String,
    pub effort: Option<Effort>,
}

/// One row in the model catalog shown by [`ModelPicker`](crate::widgets::ModelPicker).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCatalogEntry {
    pub provider: String,
    pub model: String,
    pub detail: Option<String>,
    /// Allowed reasoning efforts for this model. Empty means effort does not apply.
    pub efforts: Vec<Effort>,
}

impl ModelCatalogEntry {
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            detail: None,
            efforts: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    #[must_use]
    pub fn with_efforts(mut self, efforts: Vec<Effort>) -> Self {
        self.efforts = efforts;
        self
    }
}
