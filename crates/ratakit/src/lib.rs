//! **ratakit** — reusable ratatui components plus an optional agentic-coding UI layer.
//!
//! See the [repository README](https://github.com/jbt95/ratakit) for status and roadmap.

pub use ratakit_core as core;

#[cfg(feature = "widgets")]
pub use ratakit_widgets as widgets;

#[cfg(any(feature = "markdown", feature = "highlight-syntect", feature = "diff"))]
pub use ratakit_text as text;

#[cfg(feature = "agent")]
pub use ratakit_agent as agent;

#[cfg(feature = "acp")]
pub use ratakit_acp as acp;
