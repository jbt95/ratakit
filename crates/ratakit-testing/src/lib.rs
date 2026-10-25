//! Headless test utilities for ratakit components and widgets.
//!
//! Use [`ComponentHarness`] and [`KeySeq`] to drive event-driven state without a
//! terminal. Use [`render_widget`] or [`render_stateful`] for text snapshots,
//! [`render_stateful_buffer`] for cell/style assertions, and
//! [`assert_accessible_labels`] for text-first output. These helpers use
//! Ratatui's `TestBackend` and do not start a real terminal session.

mod accessibility;
mod buffer_text;
mod component_harness;
mod key_seq;
#[macro_use]
mod macros;
mod snapshot;

pub use accessibility::assert_palette_text_contrast;
pub use buffer_text::buffer_to_string;
pub use component_harness::ComponentHarness;
pub use key_seq::{KeySeq, KeySeqError};
pub use snapshot::{
    assert_accessible_labels, assert_stateful_widget_snapshot, assert_widget_snapshot,
    render_stateful, render_stateful_buffer, render_stateful_widget, render_to_string,
    render_widget,
};
