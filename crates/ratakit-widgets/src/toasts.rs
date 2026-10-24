use ratakit_core::{Theme, ToastLevel, ToastQueue};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::paragraph::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::theme_util::toast_style;

/// Non-modal toast stack (top of the area, most recent at the bottom of the stack).
pub struct Toasts<'a> {
    theme: &'a Theme,
    max_visible: u16,
}

impl<'a> Toasts<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            theme,
            max_visible: 5,
        }
    }

    pub fn max_visible(mut self, max: u16) -> Self {
        self.max_visible = max;
        self
    }
}

impl StatefulWidget for Toasts<'_> {
    type State = ToastQueue;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.is_empty() || state.is_empty() {
            return;
        }

        let visible = state.toasts().len().min(self.max_visible as usize);
        let start = state.toasts().len() - visible;
        let mut row = area.y;

        for toast in state.toasts().iter().skip(start) {
            if row >= area.y + area.height {
                break;
            }
            let prefix = if self.theme.screen_reader {
                format!("{}:", level_label(toast.level))
            } else {
                level_glyph(self.theme, toast.level).to_string()
            };
            let line_area = Rect {
                x: area.x,
                y: row,
                width: area.width,
                height: 1,
            };
            let text = truncate_to_width(
                format!("{prefix} {message}", message = toast.message),
                area.width as usize,
            );
            let style = toast_style(self.theme, toast.level);
            Paragraph::new(Line::from(Span::styled(text, style))).render(line_area, buf);
            row += 1;
        }
    }
}

fn level_label(level: ToastLevel) -> &'static str {
    match level {
        ToastLevel::Info => "Info",
        ToastLevel::Success => "Success",
        ToastLevel::Warning => "Warning",
        ToastLevel::Error => "Error",
    }
}

fn level_glyph(theme: &Theme, level: ToastLevel) -> char {
    match theme.glyphs {
        ratakit_core::GlyphSet::Ascii => match level {
            ToastLevel::Info => 'i',
            ToastLevel::Success => '+',
            ToastLevel::Warning => '!',
            ToastLevel::Error => 'x',
        },
        ratakit_core::GlyphSet::Unicode => match level {
            ToastLevel::Info => 'ℹ',
            ToastLevel::Success => '✓',
            ToastLevel::Warning => '⚠',
            ToastLevel::Error => '✕',
        },
    }
}

fn truncate_to_width(s: String, max: usize) -> String {
    if s.width() <= max {
        return s;
    }
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > max.saturating_sub(1) {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('…');
    out
}
