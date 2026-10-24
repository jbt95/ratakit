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

impl Effort {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::ExtraHigh => "xhigh",
        }
    }

    pub fn parse(label: &str) -> Option<Self> {
        match label {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "xhigh" | "extra_high" | "extrahigh" => Some(Self::ExtraHigh),
            _ => None,
        }
    }

    fn sort_key(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::ExtraHigh => 3,
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

/// One row in a model catalog (provider, model id, optional detail, allowed efforts).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ModelCatalogEntry {
    pub provider: String,
    pub model: String,
    pub detail: Option<String>,
    /// Allowed reasoning efforts. Empty means effort does not apply to this model.
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

/// A selectable model entry for the model picker.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ModelChoice {
    pub provider: String,
    pub model: String,
    pub effort: Option<Effort>,
}
