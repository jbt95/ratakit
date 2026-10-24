//! Generic widgets built on ratatui-core and ratakit-text.

#![allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]

mod code_block;
mod styled_convert;

pub use code_block::{CodeBlock, CodeBlockState};
pub use styled_convert::{
    semantic_to_color, styled_line_to_ratatui, styled_lines_to_ratatui, styled_span_to_ratatui,
    text_style_to_ratatui,
};
