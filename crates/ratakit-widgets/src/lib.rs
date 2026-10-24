//! Generic widgets built on ratatui-core and [`ratakit_core`] state types.

mod style;
mod virtual_list;

pub use style::{fg_style, muted_style, selected_row_style, semantic_color};
pub use virtual_list::{RowRenderer, VirtualList};
