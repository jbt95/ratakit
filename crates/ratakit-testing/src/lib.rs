//! Test utilities: render widgets to text snapshots and parse key sequences.

mod buffer_text;
mod key_seq;
#[macro_use]
mod macros;
mod snapshot;

pub use buffer_text::buffer_to_string;
pub use key_seq::{KeySeq, KeySeqError};
pub use snapshot::{
    assert_stateful_widget_snapshot, assert_widget_snapshot, render_stateful_widget,
    render_to_string, render_widget,
};
