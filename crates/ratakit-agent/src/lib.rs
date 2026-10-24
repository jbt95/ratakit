//! Agent transcript, composer, and related widgets.

#![allow(missing_docs)]

pub mod model;

pub mod state {
    pub const STUB: &str = "agent state lands in milestone 3";
}

#[cfg(feature = "widgets")]
pub mod widgets;

pub use model::{Effort, ModelCatalogEntry, ModelChoice};
