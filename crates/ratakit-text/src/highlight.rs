//! Syntax highlighting trait and language identifiers.

use std::path::Path;

use crate::styled::{StyledSpan, TextStyle};

/// Language token resolved by a [`Highlighter`] (syntect name or extension).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LangId(pub String);

/// Per-line incremental highlight state (opaque to callers).
#[derive(Debug, Default)]
pub struct HlState {
    pub(crate) inner: HlStateInner,
}

#[derive(Debug, Default)]
pub(crate) enum HlStateInner {
    #[default]
    Empty,
    #[cfg(feature = "highlight-syntect")]
    Syntect(Box<crate::highlight_syntect::SyntectHlState>),
}

/// Highlights source lines incrementally (for streaming code fences).
pub trait Highlighter: Send + Sync {
    /// Highlight one line using language-specific incremental state.
    fn highlight_line(&self, state: &mut HlState, line: &str) -> Vec<StyledSpan>;

    /// Resolve a language from a file path (extension / special names).
    fn language_for_path(&self, path: &Path) -> Option<LangId>;

    /// Resolve a language from a markdown info string or shebang hint.
    fn language_for_name(&self, name: &str) -> Option<LangId>;

    /// Create fresh highlight state for `lang`.
    fn new_state(&self, lang: LangId) -> HlState;
}

/// Plain highlighter: no syntax colors, only optional base style.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlainHighlighter {
    pub base: TextStyle,
}

impl Highlighter for PlainHighlighter {
    fn highlight_line(&self, _state: &mut HlState, line: &str) -> Vec<StyledSpan> {
        vec![StyledSpan::styled(line, self.base)]
    }

    fn language_for_path(&self, _path: &Path) -> Option<LangId> {
        None
    }

    fn language_for_name(&self, _name: &str) -> Option<LangId> {
        None
    }

    fn new_state(&self, _lang: LangId) -> HlState {
        HlState::default()
    }
}

#[cfg(feature = "highlight-syntect")]
pub use crate::highlight_syntect::SyntectHighlighter;
