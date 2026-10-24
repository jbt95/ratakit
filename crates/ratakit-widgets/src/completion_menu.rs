// Derived from openai/codex codex-rs/tui/src/bottom_pane/command_popup.rs (Apache-2.0).
// Copyright 2025 OpenAI. Licensed under the Apache License, Version 2.0.
// Modifications Copyright 2026 The ratakit Authors.

use ratakit_core::{
    CompletionMenuWidth, CompletionPlacement, CompletionState, FuzzyRankedMatch, Theme,
};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Modifier;
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::block::Block;
use ratatui_widgets::borders::{BorderType, Borders};

use crate::style::{accent, border, fg, muted, palette_bg, selected_row_style};
use crate::textarea::TextAreaState;

/// Layout data produced when anchoring a completion menu to a [`TextArea`](super::TextArea).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompletionMenuLayout {
    pub placement: CompletionPlacement,
    pub popup_area: Rect,
    pub cursor: (u16, u16),
}

/// Cached layout from the last render (popup geometry).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompletionMenuState {
    pub layout: CompletionMenuLayout,
}

/// Popup list anchored to the text-area cursor (commands, `@` files, `#` tags).
pub struct CompletionMenu<'a> {
    theme: &'a Theme,
    textarea: &'a mut TextAreaState,
    completion: &'a CompletionState,
    width: CompletionMenuWidth,
    max_rows: u16,
    textarea_max_height: Option<u16>,
    textarea_bordered: bool,
}

impl<'a> CompletionMenu<'a> {
    pub fn new(
        theme: &'a Theme,
        textarea: &'a mut TextAreaState,
        completion: &'a CompletionState,
    ) -> Self {
        Self {
            theme,
            textarea,
            completion,
            width: CompletionMenuWidth::Full,
            max_rows: 6,
            textarea_max_height: None,
            textarea_bordered: true,
        }
    }

    pub fn max_rows(mut self, rows: u16) -> Self {
        self.max_rows = rows.max(1);
        self
    }

    pub fn textarea_max_height(mut self, height: Option<u16>) -> Self {
        self.textarea_max_height = height;
        self
    }

    pub fn textarea_bordered(mut self, bordered: bool) -> Self {
        self.textarea_bordered = bordered;
        self
    }

    /// Computes popup geometry from the text area without drawing.
    pub fn layout_for(&mut self, area: Rect) -> CompletionMenuLayout {
        if !self.completion.is_open() || self.completion.ranked_matches().is_empty() {
            return CompletionMenuLayout::default();
        }

        let cursor_layout = self.textarea.cursor_layout_with_border(
            area,
            self.textarea_max_height,
            self.textarea_bordered,
        );
        let inner = cursor_layout.inner;
        let (cursor_x, cursor_y) = cursor_layout.cursor;

        let match_count = self
            .completion
            .ranked_matches()
            .len()
            .min(self.max_rows as usize);
        let popup_height = match_count as u16 + 2;
        let popup_width = match inner.width {
            0 => 0,
            w => match self.width {
                CompletionMenuWidth::Full => w,
            },
        };

        let space_below = inner.bottom().saturating_sub(cursor_y + 1);
        let space_above = cursor_y.saturating_sub(inner.y);

        let placement = if space_below >= popup_height || space_below >= space_above {
            CompletionPlacement::Below
        } else {
            CompletionPlacement::Above
        };

        let popup_y = match placement {
            CompletionPlacement::Below => cursor_y + 1,
            CompletionPlacement::Above => cursor_y.saturating_sub(popup_height),
        };

        let popup_area = Rect {
            x: inner.x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        CompletionMenuLayout {
            placement,
            popup_area,
            cursor: (cursor_x, cursor_y),
        }
    }
}

impl StatefulWidget for CompletionMenu<'_> {
    type State = CompletionMenuState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.layout = self.layout_for(area);
        let popup = state.layout.popup_area;
        if popup.width < 2 || popup.height < 2 {
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border(self.theme))
            .style(palette_bg(self.theme));
        let inner = block.inner(popup);
        block.render(popup, buf);

        let ranked = self.completion.ranked_matches();
        let selected = self.completion.selected_index();
        let scroll = self.completion.list_scroll_offset();
        let visible_end = (scroll + inner.height as usize).min(ranked.len());

        for (view_row, match_idx) in (scroll..visible_end).enumerate() {
            let y = inner.y + view_row as u16;
            if y >= inner.bottom() {
                break;
            }
            let ranked_match = &ranked[match_idx];
            let is_selected = match_idx == selected;
            let item = self
                .completion
                .catalog()
                .get(ranked_match.item_index)
                .expect("ranked index");
            render_row(
                buf,
                RowDraw {
                    x: inner.x,
                    y,
                    width: inner.width,
                    label: item.label.as_str(),
                    detail: item.detail.as_deref(),
                    ranked: ranked_match,
                    selected: is_selected,
                    theme: self.theme,
                },
            );
        }
    }
}

struct RowDraw<'a> {
    x: u16,
    y: u16,
    width: u16,
    label: &'a str,
    detail: Option<&'a str>,
    ranked: &'a FuzzyRankedMatch,
    selected: bool,
    theme: &'a Theme,
}

fn render_row(buf: &mut Buffer, row: RowDraw<'_>) {
    let base_style = if row.selected {
        selected_row_style(row.theme)
    } else {
        fg(row.theme)
    };
    let highlight = accent(row.theme).add_modifier(Modifier::BOLD);

    let label_width = row.width.saturating_sub(2) as usize;
    let mut col = 0usize;
    for (idx, ch) in row.label.chars().enumerate() {
        if col >= label_width {
            break;
        }
        let style = if row.ranked.indices.contains(&(idx as u32)) {
            highlight
        } else {
            base_style
        };
        let cell_x = row.x + col as u16;
        if cell_x < row.x + row.width {
            buf[(cell_x, row.y)].set_symbol(&ch.to_string());
            buf[(cell_x, row.y)].set_style(style);
        }
        col += 1;
    }

    if let Some(detail) = row.detail {
        let detail_style = muted(row.theme);
        let detail_x = row.x + col as u16 + 1;
        if detail_x < row.x + row.width {
            let room = (row.x + row.width - detail_x) as usize;
            let clipped: String = detail.chars().take(room).collect();
            buf.set_string(detail_x, row.y, clipped, detail_style);
        }
    }
}
