//! Generic widgets built on ratatui-core and [`ratakit_core`] state types.

mod style;
mod textarea;
mod virtual_list;
mod wrap;

pub use style::{
    accent, border, cursor_style, fg, muted, palette_bg, selected_row_style, semantic_color,
};
pub use textarea::{TextArea, TextAreaState, TextIntent};
pub use virtual_list::{RowRenderer, VirtualList};
