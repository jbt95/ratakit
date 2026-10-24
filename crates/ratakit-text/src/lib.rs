//! Text processing for ratakit: styled lines, wrapping, and syntax highlighting.
//!
//! This crate deliberately has **no** ratatui dependency; widgets in `ratakit-widgets`
//! convert [`StyledLine`] values into ratatui types at render time.

#![allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]

mod highlight;
mod styled;
mod wrap;

#[cfg(feature = "diff")]
mod diff;

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

#[cfg(feature = "diff")]
pub use diff::{
    DiffHunk, DiffLine, DiffModel, DiffParseError, DiffSide, FileDiff, IntraSpan, display_path,
    enrich_intra_line, path_for_side,
};
#[cfg(feature = "diff")]
pub use similar::ChangeTag;
