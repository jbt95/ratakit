//! Test utilities: render widgets to text snapshots and parse key sequences.

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
