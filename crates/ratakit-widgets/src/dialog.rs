use ratakit_core::DialogState;
use ratakit_core::Theme;
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui_core::style::{Modifier, Style};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::block::Block;
use ratatui_widgets::borders::BorderType;
use ratatui_widgets::clear::Clear;
use ratatui_widgets::paragraph::Paragraph;

use crate::style::{accent, border, cursor_style, error, fg, muted, palette_bg};

/// Action button description for [`Dialog`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogButton {
    pub label: String,
    pub primary: bool,
}

impl DialogButton {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            primary: false,
        }
    }

    pub fn primary(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            primary: true,
        }
    }
}

/// Centered dialog with title, body, and a row of buttons.
pub struct Dialog<'a> {
    theme: &'a Theme,
    title: &'a str,
    message: &'a str,
    buttons: &'a [DialogButton],
}

impl<'a> Dialog<'a> {
    pub fn new(
        theme: &'a Theme,
        title: &'a str,
        message: &'a str,
        buttons: &'a [DialogButton],
    ) -> Self {
        Self {
            theme,
            title,
            message,
            buttons,
        }
    }
}

impl StatefulWidget for Dialog<'_> {
    type State = DialogState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.set_button_count(self.buttons.len());
        if !state.is_open() {
            return;
        }

        let max_w = area.width.clamp(20, 60);
        let max_h = area.height.clamp(6, 12);
        let dialog_area = centered_rect(max_w, max_h, area);

        Clear.render(dialog_area, buf);

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(self.title)
            .style(palette_bg(self.theme).patch(border(self.theme)));
        let inner = block.inner(dialog_area);
        block.render(dialog_area, buf);

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

        Paragraph::new(self.message)
            .style(fg(self.theme))
            .render(chunks[1], buf);

        render_buttons(
            self.theme,
            self.buttons,
            state.selected_button(),
            chunks[2],
            buf,
        );
    }
}

/// Full-screen modal overlay with a centered [`Dialog`].
pub struct Modal<'a> {
    dialog: Dialog<'a>,
}

impl<'a> Modal<'a> {
    pub fn new(dialog: Dialog<'a>) -> Self {
        Self { dialog }
    }
}

impl StatefulWidget for Modal<'_> {
    type State = DialogState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.is_open() {
            return;
        }
        // Dim backdrop
        let dim = Style::default().bg(ratatui_core::style::Color::Rgb(0, 0, 0));
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = &mut buf[(x, y)];
                cell.set_bg(dim.bg.unwrap_or(ratatui_core::style::Color::Black));
            }
        }
        StatefulWidget::render(self.dialog, area, buf, state);
    }
}

fn render_buttons(
    theme: &Theme,
    buttons: &[DialogButton],
    selected: usize,
    area: Rect,
    buf: &mut Buffer,
) {
    if buttons.is_empty() || area.height == 0 {
        return;
    }

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, btn) in buttons.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let label = format!("[{}] {}", i + 1, btn.label);
        let style = if i == selected {
            cursor_style(theme)
        } else if btn.primary {
            accent(theme).add_modifier(Modifier::BOLD)
        } else {
            fg(theme)
        };
        spans.push(Span::styled(label, style));
    }

    let line = Line::from(spans).alignment(Alignment::Center);
    Paragraph::new(line).render(area, buf);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let horizontal = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width),
        Constraint::Fill(1),
    ])
    .flex(Flex::Center)
    .split(area);
    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .flex(Flex::Center)
    .split(horizontal[1]);
    vertical[1]
}

/// Confirmation dialog with Cancel + OK (destructive styling on title when `dangerous`).
pub struct ConfirmDialog<'a> {
    theme: &'a Theme,
    title: &'a str,
    message: &'a str,
    dangerous: bool,
}

impl<'a> ConfirmDialog<'a> {
    pub fn new(theme: &'a Theme, title: &'a str, message: &'a str) -> Self {
        Self {
            theme,
            title,
            message,
            dangerous: false,
        }
    }

    pub fn dangerous(self) -> Self {
        Self {
            dangerous: true,
            ..self
        }
    }

    fn buttons(&self) -> [DialogButton; 2] {
        [
            DialogButton::new("Cancel"),
            DialogButton::primary(if self.dangerous { "Delete" } else { "OK" }),
        ]
    }
}

impl StatefulWidget for ConfirmDialog<'_> {
    type State = DialogState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let buttons = self.buttons();
        let title_style = if self.dangerous {
            error(self.theme)
        } else {
            fg(self.theme)
        };
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(Line::from(self.title).style(title_style))
            .style(palette_bg(self.theme).patch(border(self.theme)));

        state.set_button_count(buttons.len());
        if !state.is_open() {
            return;
        }

        let max_w = area.width.clamp(20, 50);
        let max_h = area.height.clamp(6, 10);
        let dialog_area = centered_rect(max_w, max_h, area);
        Clear.render(dialog_area, buf);

        let inner = block.inner(dialog_area);
        block.render(dialog_area, buf);

        let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);

        Paragraph::new(self.message)
            .style(muted(self.theme))
            .render(chunks[0], buf);
        render_buttons(
            self.theme,
            &buttons,
            state.selected_button(),
            chunks[1],
            buf,
        );
    }
}
