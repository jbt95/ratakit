use ratakit_core::{KeyHint, Theme};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;
use ratatui_widgets::paragraph::Paragraph;

use crate::theme_util::{accent_style, muted_style};

/// Renders key chords and labels (typically from [`ratakit_core::key_hints_from_keymap`]).
pub struct KeyHints<'a> {
    theme: &'a Theme,
    hints: &'a [KeyHint],
    separator: &'a str,
}

impl<'a> KeyHints<'a> {
    pub fn new(theme: &'a Theme, hints: &'a [KeyHint]) -> Self {
        Self {
            theme,
            hints,
            separator: "  ",
        }
    }

    pub fn separator(mut self, separator: &'a str) -> Self {
        self.separator = separator;
        self
    }
}

impl Widget for KeyHints<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() || self.hints.is_empty() {
            return;
        }

        let mut spans = Vec::new();
        for (i, hint) in self.hints.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(self.separator, muted_style(self.theme)));
            }
            spans.push(Span::styled(
                format!("[{}]", hint.chord),
                accent_style(self.theme),
            ));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(hint.label.as_str(), muted_style(self.theme)));
        }
        Paragraph::new(Line::from(spans)).render(area, buf);
    }
}
