use std::time::Instant;

use ratakit_core::{
    Component, Event, KeyCode, KeyEvent, Keymap, Outcome, PasteBurst, Revisioned, Scope,
    TextBuffer, Theme,
};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::block::Block;
use ratatui_widgets::borders::{BorderType, Borders};
use ratatui_widgets::paragraph::Paragraph;

use crate::style::{accent, border, cursor_style, fg, muted, palette_bg};
use crate::wrap::{WrappedLine, cursor_wrapped_position, wrap_lines};

/// High-level intents produced by [`TextAreaState`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextIntent {
    Submit { text: String },
}

/// Stateful multiline input backed by [`TextBuffer`].
#[derive(Debug, Clone)]
pub struct TextAreaState {
    pub buffer: TextBuffer,
    paste_burst: PasteBurst,
    scroll_top: usize,
    last_width: u16,
}

impl Default for TextAreaState {
    fn default() -> Self {
        Self::new()
    }
}

impl TextAreaState {
    pub fn new() -> Self {
        Self {
            buffer: TextBuffer::new(),
            paste_burst: PasteBurst::new(),
            scroll_top: 0,
            last_width: 80,
        }
    }

    pub fn with_buffer(buffer: TextBuffer) -> Self {
        Self {
            buffer,
            paste_burst: PasteBurst::new(),
            scroll_top: 0,
            last_width: 80,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.display_len() == 0
    }

    fn wrapped(&self, width: u16) -> Vec<WrappedLine> {
        wrap_lines(&self.buffer.display(), width)
    }

    fn ensure_cursor_visible(&mut self, width: u16, height: u16) {
        let lines = self.wrapped(width);
        let (row, _) = cursor_wrapped_position(&lines, self.buffer.cursor());
        if height == 0 {
            self.scroll_top = row;
            return;
        }
        if row < self.scroll_top {
            self.scroll_top = row;
        } else if row >= self.scroll_top + height as usize {
            self.scroll_top = row.saturating_sub(height.saturating_sub(1) as usize);
        }
    }

    /// Inner content rect and absolute cursor coordinates for anchoring overlays.
    pub fn cursor_layout(&mut self, area: Rect, max_height: Option<u16>) -> TextAreaCursorLayout {
        self.cursor_layout_with_border(area, max_height, true)
    }

    /// Like [`Self::cursor_layout`], with geometry matching a borderless text area.
    pub fn cursor_layout_with_border(
        &mut self,
        area: Rect,
        max_height: Option<u16>,
        bordered: bool,
    ) -> TextAreaCursorLayout {
        let horizontal_chrome = if bordered { 2 } else { 0 };
        let vertical_chrome = if bordered { 2 } else { 1 };
        let inner_width = area.width.saturating_sub(horizontal_chrome);
        let inner_height = area
            .height
            .saturating_sub(vertical_chrome)
            .min(max_height.unwrap_or(area.height));
        self.last_width = inner_width;
        self.ensure_cursor_visible(inner_width, inner_height);

        let inner = Rect {
            x: area.x + if bordered { 1 } else { 0 },
            y: area.y + 1,
            width: inner_width,
            height: inner_height,
        };

        let wrapped = self.wrapped(inner_width);
        let (cursor_row, cursor_col) = cursor_wrapped_position(&wrapped, self.buffer.cursor());
        let view_row = cursor_row.saturating_sub(self.scroll_top);
        let cursor_x = inner.x + cursor_col.min(inner.width as usize - 1) as u16;
        let cursor_y = inner.y + view_row.min(inner.height as usize - 1) as u16;

        TextAreaCursorLayout {
            inner,
            cursor: (cursor_x, cursor_y),
        }
    }
}

/// Layout snapshot for anchoring widgets (e.g. [`CompletionMenu`](super::CompletionMenu)) to the caret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextAreaCursorLayout {
    pub inner: Rect,
    pub cursor: (u16, u16),
}

impl Component for TextAreaState {
    type Intent = TextIntent;

    fn handle(&mut self, ev: &Event, keys: &Keymap) -> Outcome<Self::Intent> {
        let now = Instant::now();
        match ev {
            Event::Paste(text) => {
                self.paste_burst.on_paste(now);
                self.buffer.insert_paste(text);
                Outcome::Consumed
            }
            Event::Key(key) => self.handle_key(key, keys, now),
            Event::Tick(t) => {
                self.paste_burst.tick(*t);
                Outcome::Ignored
            }
            _ => Outcome::Ignored,
        }
    }

    fn revision(&self) -> u64 {
        self.buffer.revision()
    }
}

impl TextAreaState {
    fn handle_key(&mut self, key: &KeyEvent, keys: &Keymap, now: Instant) -> Outcome<TextIntent> {
        if let Some(action) = keys.resolve(&[Scope::Focused, Scope::Global], key) {
            if action.0 == "textarea.submit" {
                let text = self.buffer.text();
                self.paste_burst.reset();
                return Outcome::Intent(TextIntent::Submit { text });
            }
        }

        match key.code {
            KeyCode::Enter => {
                if self.paste_burst.enter_inserts_newline(now) {
                    self.buffer.insert_str("\n");
                    return Outcome::Consumed;
                }
                let text = self.buffer.text();
                self.paste_burst.reset();
                Outcome::Intent(TextIntent::Submit { text })
            }
            KeyCode::Char(c) => {
                let mods = key.modifiers;
                if mods.control && c == 'z' && self.buffer.undo() {
                    return Outcome::Consumed;
                }
                if mods.control && (c == 'y' || c == 'Z') && self.buffer.redo() {
                    return Outcome::Consumed;
                }
                self.paste_burst.on_char(now);
                self.buffer.insert_str(&c.to_string());
                Outcome::Consumed
            }
            KeyCode::Backspace => {
                self.paste_burst.on_other_key(now);
                self.buffer.delete_backward();
                Outcome::Consumed
            }
            KeyCode::Delete => {
                self.paste_burst.on_other_key(now);
                self.buffer.delete_forward();
                Outcome::Consumed
            }
            KeyCode::Left => {
                self.paste_burst.on_other_key(now);
                self.buffer.move_left();
                Outcome::Consumed
            }
            KeyCode::Right => {
                self.paste_burst.on_other_key(now);
                self.buffer.move_right();
                Outcome::Consumed
            }
            KeyCode::Up => {
                self.paste_burst.on_other_key(now);
                self.buffer.move_up();
                Outcome::Consumed
            }
            KeyCode::Down => {
                self.paste_burst.on_other_key(now);
                self.buffer.move_down();
                Outcome::Consumed
            }
            _ => {
                self.paste_burst.on_other_key(now);
                Outcome::Ignored
            }
        }
    }
}

/// Multiline text input widget.
pub struct TextArea<'a> {
    theme: &'a Theme,
    placeholder: Option<&'a str>,
    max_height: Option<u16>,
    block_title: Option<&'a str>,
    bordered: bool,
}

impl<'a> TextArea<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            theme,
            placeholder: None,
            max_height: None,
            block_title: None,
            bordered: true,
        }
    }

    pub fn placeholder(mut self, text: &'a str) -> Self {
        self.placeholder = Some(text);
        self
    }

    pub fn max_height(mut self, height: u16) -> Self {
        self.max_height = Some(height);
        self
    }

    pub fn block_title(mut self, title: &'a str) -> Self {
        self.block_title = Some(title);
        self
    }

    /// Render a text label instead of a box border when false.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }
}

impl Widget for TextArea<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = TextAreaState::new();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}

impl StatefulWidget for TextArea<'_> {
    type State = TextAreaState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let horizontal_chrome = if self.bordered { 2 } else { 0 };
        let vertical_chrome = if self.bordered { 2 } else { 1 };
        if area.width <= horizontal_chrome || area.height <= vertical_chrome {
            return;
        }

        let inner_width = area.width.saturating_sub(horizontal_chrome);
        let inner_height = area
            .height
            .saturating_sub(vertical_chrome)
            .min(self.max_height.unwrap_or(area.height));

        state.last_width = inner_width;
        state.ensure_cursor_visible(inner_width, inner_height);

        let title = self.block_title.unwrap_or("composer");
        let inner = if self.bordered {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border(self.theme))
                .style(palette_bg(self.theme))
                .title(title)
                .title_style(accent(self.theme));
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            Paragraph::new(format!("{title}:"))
                .style(accent(self.theme))
                .render(Rect::new(area.x, area.y, area.width, 1), buf);
            Rect::new(area.x, area.y + 1, area.width, area.height - 1)
        };

        let wrapped = state.wrapped(inner_width);
        let (cursor_row, cursor_col) = cursor_wrapped_position(&wrapped, state.buffer.cursor());

        let show_placeholder = state.is_empty() && self.placeholder.is_some();
        let visible_start = state.scroll_top;
        let visible_end = (visible_start + inner.height as usize).min(wrapped.len());

        for (view_row, line_idx) in (visible_start..visible_end).enumerate() {
            let y = inner.y + view_row as u16;
            if y >= inner.bottom() {
                break;
            }
            let line = &wrapped[line_idx];
            let style = if show_placeholder && line_idx == 0 {
                muted(self.theme)
            } else {
                fg(self.theme)
            };
            let text = if show_placeholder && line_idx == 0 {
                self.placeholder.unwrap_or("")
            } else {
                line.text.as_str()
            };
            write_line(buf, inner.x, y, inner.width, text, style);

            if !show_placeholder && line_idx == cursor_row {
                let x = inner.x + cursor_col.min(inner.width as usize - 1) as u16;
                if x < inner.right() {
                    let cell = buf[(x, y)].clone();
                    let ch = cell.symbol().chars().next().unwrap_or(' ');
                    buf[(x, y)].set_symbol(&ch.to_string());
                    buf[(x, y)].set_style(cursor_style(self.theme));
                }
            }
        }
    }
}

fn write_line(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    let clipped: String = text.chars().take(width as usize).collect();
    buf.set_string(x, y, clipped, style);
}
