use ratakit_core::{Motion, Theme};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Modifier;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::paragraph::Paragraph;

use crate::theme_util::{accent_style, fg_style};

/// Animated (or static) activity indicator.
///
/// State lives in [`Motion`] on the core side; sync [`Theme::motion`] via [`Motion::set_mode`] each frame.
pub struct Spinner<'a> {
    theme: &'a Theme,
    label: Option<&'a str>,
}

impl<'a> Spinner<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self { theme, label: None }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

impl StatefulWidget for Spinner<'_> {
    type State = Motion;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.set_screen_reader(self.theme.screen_reader);
        state.set_mode(self.theme.motion);
        if area.is_empty() {
            return;
        }

        let indicator = state.indicator(self.theme);
        let mut spans = vec![Span::styled(
            format!("{indicator} "),
            accent_style(self.theme).add_modifier(Modifier::BOLD),
        )];
        if let Some(label) = self.label {
            spans.push(Span::styled(label, fg_style(self.theme)));
        }
        Paragraph::new(Line::from(spans)).render(area, buf);
    }
}
