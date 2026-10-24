//! Styled text primitives independent of ratatui.

use ratakit_core::SemanticColor;
use unicode_width::UnicodeWidthStr;

/// Text modifiers (bold, italic, …) carried on [`TextStyle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextModifier(u8);

impl TextModifier {
    pub const BOLD: Self = Self(1 << 0);
    pub const ITALIC: Self = Self(1 << 1);
    pub const UNDERLINE: Self = Self(1 << 2);
    pub const DIM: Self = Self(1 << 3);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for TextModifier {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for TextModifier {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Foreground/background styling using ratakit semantic colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextStyle {
    pub fg: Option<SemanticColor>,
    pub bg: Option<SemanticColor>,
    pub modifier: TextModifier,
}

impl TextStyle {
    /// Unstyled text (inherits terminal defaults when rendered).
    pub const PLAIN: Self = Self {
        fg: None,
        bg: None,
        modifier: TextModifier::empty(),
    };

    /// Create a style with the given foreground color.
    pub fn fg(color: SemanticColor) -> Self {
        Self {
            fg: Some(color),
            bg: None,
            modifier: TextModifier::empty(),
        }
    }

    /// Set foreground.
    pub fn with_fg(self, color: SemanticColor) -> Self {
        Self {
            fg: Some(color),
            ..self
        }
    }

    /// Set background.
    pub fn with_bg(self, color: SemanticColor) -> Self {
        Self {
            bg: Some(color),
            ..self
        }
    }

    /// Add modifiers (merged with existing flags).
    pub fn with_modifier(self, modifier: TextModifier) -> Self {
        Self {
            modifier: self.modifier | modifier,
            ..self
        }
    }

    /// Merge `other` on top of this style (`other` wins for set fields).
    pub fn patch(self, other: Self) -> Self {
        Self {
            fg: other.fg.or(self.fg),
            bg: other.bg.or(self.bg),
            modifier: self.modifier | other.modifier,
        }
    }
}

/// A run of text with a single style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledSpan {
    pub text: String,
    pub style: TextStyle,
    /// OSC 8 hyperlink destination when rendered in a terminal (visible text is the span's `text` field).
    pub link: Option<String>,
}

impl StyledSpan {
    /// Plain span with default style.
    pub fn raw(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::PLAIN,
            link: None,
        }
    }

    /// Span with explicit style.
    pub fn styled(text: impl Into<String>, style: TextStyle) -> Self {
        Self {
            text: text.into(),
            style,
            link: None,
        }
    }

    /// Hyperlink span (underline styling should be applied separately).
    pub fn link(text: impl Into<String>, style: TextStyle, destination: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style,
            link: Some(destination.into()),
        }
    }

    /// Display width in terminal columns.
    pub fn width(&self) -> usize {
        self.text.width()
    }
}

/// One terminal row of styled spans.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StyledLine {
    pub spans: Vec<StyledSpan>,
}

impl StyledLine {
    /// Empty line.
    pub fn new() -> Self {
        Self::default()
    }

    /// Single plain-text span.
    pub fn raw(text: impl Into<String>) -> Self {
        Self {
            spans: vec![StyledSpan::raw(text)],
        }
    }

    /// Push a span onto the line.
    pub fn push(&mut self, span: StyledSpan) {
        self.spans.push(span);
    }

    /// Total display width.
    pub fn width(&self) -> usize {
        self.spans.iter().map(StyledSpan::width).sum()
    }

    /// Concatenate span text (ignores styling).
    pub fn plain_text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }
}
