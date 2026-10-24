//! Agent transcript, composer, and related widgets.
//!
//! The [`state`] module is ratatui-free and can be used from other UIs (for example GPUI).

#![allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]

pub mod state;

#[cfg(feature = "widgets")]
pub mod widgets;
