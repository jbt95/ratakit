//! Text processing for ratakit: styled lines, wrapping, and syntax highlighting.
//!
//! This crate deliberately has **no** ratatui dependency; widgets in `ratakit-widgets`
//! convert [`StyledLine`] values into ratatui types at render time.

#![allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]

mod highlight;
mod styled;
mod wrap;

#[cfg(feature = "markdown")]
mod markdown_doc;
#[cfg(feature = "markdown")]
mod markdown_render;
#[cfg(feature = "markdown")]
mod markdown_stream;

#[cfg(feature = "highlight-syntect")]
mod highlight_syntect;

pub use highlight::{Highlighter, HlState, LangId, PlainHighlighter};
pub use styled::{StyledLine, StyledSpan, TextModifier, TextStyle};
pub use wrap::{
    PerLineWrapCache, WrapCache, WrappedLine, cursor_wrapped_position, wrap_lines,
    wrap_styled_line, wrap_styled_lines,
};

#[cfg(feature = "highlight-syntect")]
pub use highlight::SyntectHighlighter;
#[cfg(feature = "highlight-syntect")]
pub use highlight_syntect::{SyntectHlState, language_from_shebang};

#[cfg(feature = "markdown")]
pub use markdown_doc::MarkdownDoc;
#[cfg(feature = "markdown")]
pub use markdown_render::{MarkdownOptions, MarkdownStyles, StreamingCodeState, render_markdown};
#[cfg(feature = "markdown")]
pub use markdown_stream::MarkdownStream;
