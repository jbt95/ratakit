use ratakit_core::{
    CheckboxState, GlyphSet, RadioGroupState, SelectState, SwitchState, Theme, ToggleState,
};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Modifier, Style};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::paragraph::Paragraph;

use crate::style::{accent, border, cursor_style, fg, muted, semantic_color};

/// Checkbox with caller-owned activation and a reusable [`CheckboxState`].
pub struct Checkbox<'a> {
    theme: &'a Theme,
    label: &'a str,
}

impl<'a> Checkbox<'a> {
    pub fn new(theme: &'a Theme, label: &'a str) -> Self {
        Self { theme, label }
    }
}

impl StatefulWidget for Checkbox<'_> {
    type State = CheckboxState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        render_toggle(self.theme, self.label, state, area, buf, false);
    }
}

/// Switch-style view over [`SwitchState`].
pub struct Switch<'a> {
    theme: &'a Theme,
    label: &'a str,
}

impl<'a> Switch<'a> {
    pub fn new(theme: &'a Theme, label: &'a str) -> Self {
        Self { theme, label }
    }
}

impl StatefulWidget for Switch<'_> {
    type State = SwitchState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        render_toggle(self.theme, self.label, state, area, buf, true);
    }
}

fn render_toggle(
    theme: &Theme,
    label: &str,
    state: &ToggleState,
    area: Rect,
    buf: &mut Buffer,
    switch: bool,
) {
    if area.is_empty() {
        return;
    }
    let control = if switch {
        match (theme.glyphs, state.is_checked()) {
            (GlyphSet::Ascii, true) => "[ON]",
            (GlyphSet::Ascii, false) => "[OFF]",
            (GlyphSet::Unicode, true) => "[●──]",
            (GlyphSet::Unicode, false) => "[──○]",
        }
    } else {
        match (theme.glyphs, state.is_checked()) {
            (GlyphSet::Ascii, true) => "[x]",
            (GlyphSet::Ascii, false) => "[ ]",
            (GlyphSet::Unicode, true) => "☑",
            (GlyphSet::Unicode, false) => "☐",
        }
    };
    let control_style = if state.is_disabled() {
        muted(theme).add_modifier(Modifier::DIM)
    } else if state.is_checked() {
        accent(theme).add_modifier(Modifier::BOLD)
    } else {
        border(theme)
    };
    let label_style = if state.is_disabled() {
        muted(theme).add_modifier(Modifier::DIM)
    } else {
        fg(theme)
    };
    let label = if state.is_disabled() && theme.screen_reader {
        format!("{label} (disabled)")
    } else {
        label.to_owned()
    };
    Paragraph::new(Line::from(vec![
        Span::styled(control, control_style),
        Span::raw("  "),
        Span::styled(label, label_style),
    ]))
    .render(area, buf);
}

/// Single-choice radio list. Arrow keys and Home/End update [`RadioGroupState`].
pub struct RadioGroup<'a> {
    theme: &'a Theme,
    options: &'a [&'a str],
}

impl<'a> RadioGroup<'a> {
    pub fn new(theme: &'a Theme, options: &'a [&'a str]) -> Self {
        Self { theme, options }
    }
}

impl StatefulWidget for RadioGroup<'_> {
    type State = RadioGroupState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.is_empty() {
            return;
        }
        state.set_option_count(self.options.len());
        let visible = area.height.min(self.options.len() as u16);
        for index in 0..visible as usize {
            let selected = state.selected() == index;
            let marker = match (self.theme.glyphs, selected) {
                (GlyphSet::Ascii, true) => "(*)",
                (GlyphSet::Ascii, false) => "( )",
                (GlyphSet::Unicode, true) => "●",
                (GlyphSet::Unicode, false) => "○",
            };
            let marker_style = if selected {
                accent(self.theme)
            } else {
                border(self.theme)
            };
            let label_style = if selected {
                fg(self.theme).add_modifier(Modifier::BOLD)
            } else {
                fg(self.theme)
            };
            Paragraph::new(Line::from(vec![
                Span::styled(marker, marker_style),
                Span::raw("  "),
                Span::styled(self.options[index], label_style),
            ]))
            .render(Rect::new(area.x, area.y + index as u16, area.width, 1), buf);
        }
    }
}

/// One text option for a [`Select`] control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectOption {
    pub label: String,
}

impl SelectOption {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

/// Compact single-selection field with an inline keyboard-navigable option list.
pub struct Select<'a> {
    theme: &'a Theme,
    options: &'a [SelectOption],
    placeholder: &'a str,
}

impl<'a> Select<'a> {
    pub fn new(theme: &'a Theme, options: &'a [SelectOption]) -> Self {
        Self {
            theme,
            options,
            placeholder: "Select an option",
        }
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }
}

impl StatefulWidget for Select<'_> {
    type State = SelectState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.is_empty() {
            return;
        }
        state.set_option_count(self.options.len());
        let selected_label = state
            .selected()
            .and_then(|index| self.options.get(index))
            .map(|option| option.label.as_str());
        let label = selected_label.unwrap_or(self.placeholder);
        let arrow = if self.theme.glyphs == GlyphSet::Ascii {
            "v"
        } else {
            "⌄"
        };
        let field_style = if state.is_open() {
            cursor_style(self.theme)
        } else if selected_label.is_none() {
            muted(self.theme)
        } else {
            fg(self.theme).patch(Style::default().bg(semantic_color(self.theme.palette.background)))
        };
        let field = Line::from(vec![
            Span::styled("[ ", border(self.theme)),
            Span::styled(label, field_style),
            Span::raw("  "),
            Span::styled(arrow, accent(self.theme)),
            Span::styled(" ]", border(self.theme)),
        ]);
        Paragraph::new(field).render(Rect::new(area.x, area.y, area.width, 1), buf);

        if !state.is_open() || area.height <= 1 {
            return;
        }
        let list_height = usize::from(area.height - 1).min(self.options.len());
        let highlight = state.highlighted().unwrap_or(0);
        let start = highlight
            .saturating_sub(list_height.saturating_sub(1))
            .min(self.options.len().saturating_sub(list_height));
        for (row, index) in (start..start + list_height).enumerate() {
            let is_highlighted = state.highlighted() == Some(index);
            let is_selected = state.selected() == Some(index);
            let marker = if is_highlighted {
                if self.theme.glyphs == GlyphSet::Ascii {
                    ">"
                } else {
                    "›"
                }
            } else if is_selected {
                if self.theme.glyphs == GlyphSet::Ascii {
                    "*"
                } else {
                    "✓"
                }
            } else {
                " "
            };
            let style = if is_highlighted {
                cursor_style(self.theme)
            } else {
                fg(self.theme)
            };
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{marker} "),
                    if is_highlighted {
                        accent(self.theme)
                    } else {
                        muted(self.theme)
                    },
                ),
                Span::styled(self.options[index].label.as_str(), style),
            ]))
            .render(
                Rect::new(area.x, area.y + 1 + row as u16, area.width, 1),
                buf,
            );
        }
    }
}
