//! Full-screen (default) fuzzy picker with grouped rows and optional preview pane.

use std::marker::PhantomData;

use ratakit_core::{
    ItemHeights, PickerDisplayMode, PickerItem, PickerLayoutHint, PickerRowKind,
    PickerSecondaryState, PickerState, Theme, visible_rows,
};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Alignment, Constraint, Flex, Layout, Margin, Rect};
use ratatui_core::style::{Modifier, Style};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::block::Block;
use ratatui_widgets::borders::BorderType;
use ratatui_widgets::clear::Clear;
use ratatui_widgets::paragraph::Paragraph;
use ratatui_widgets::scrollbar::{Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::style::{accent, border, cursor_style, fg, muted, palette_bg, selected_row_style};

/// Fuzzy selection overlay backed by [`PickerState`].
pub struct FuzzyPicker<'a, T: PickerItem> {
    title: &'a str,
    theme: &'a Theme,
    show_preview: bool,
    mode: PickerDisplayMode,
    show_scrollbar: bool,
    _item: PhantomData<T>,
}

impl<'a, T: PickerItem> FuzzyPicker<'a, T> {
    pub fn new(title: &'a str, theme: &'a Theme) -> Self {
        Self {
            title,
            theme,
            show_preview: false,
            mode: PickerDisplayMode::default(),
            show_scrollbar: true,
            _item: PhantomData,
        }
    }

    #[must_use]
    pub fn preview(mut self, show: bool) -> Self {
        self.show_preview = show;
        self
    }

    #[must_use]
    pub fn display_mode(mut self, mode: PickerDisplayMode) -> Self {
        self.mode = mode;
        self
    }

    #[must_use]
    pub fn scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }
}

impl<'a, T: PickerItem> StatefulWidget for FuzzyPicker<'a, T> {
    type State = PickerState<T>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.is_open() || area.width == 0 || area.height == 0 {
            return;
        }

        let panel = match self.mode {
            PickerDisplayMode::Fullscreen => area,
            PickerDisplayMode::Centered {
                max_width,
                max_height,
            } => centered_rect(max_width, max_height, area),
        };

        if !matches!(self.mode, PickerDisplayMode::Fullscreen) {
            dim_backdrop(area, buf);
            Clear.render(panel, buf);
        }

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(self.title)
            .style(palette_bg(self.theme).patch(border(self.theme)));
        let inner = block.inner(panel);
        block.render(panel, buf);

        let has_secondary = state.secondary().is_some();
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(if has_secondary { 1 } else { 0 }),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

        render_query_line(self.theme, state.query(), vertical[0], buf);
        if has_secondary {
            if let Some(secondary) = state.secondary() {
                render_secondary_axis(self.theme, secondary, vertical[1], buf);
            }
        }

        let list_area = vertical[2];
        let preview_min_width = 24u16;
        let use_preview = self.show_preview
            && list_area.width >= preview_min_width + 20
            && state.selected_detail().is_some();

        let (list_rect, preview_rect) = if use_preview {
            let cols =
                Layout::horizontal([Constraint::Min(20), Constraint::Length(preview_min_width)])
                    .split(list_area);
            (cols[0], Some(cols[1]))
        } else {
            (list_area, None)
        };

        let heights = PickerRowHeights {
            len: state.rows().len(),
            revision: state.ui_revision(),
        };
        let content_width = if self.show_scrollbar && list_rect.width > 1 {
            list_rect.width - 1
        } else {
            list_rect.width
        };
        state
            .list_state_mut()
            .prepare_layout(&heights, content_width, list_rect.height);
        state.set_layout_hint(PickerLayoutHint {
            list_width: content_width,
            list_viewport_height: list_rect.height,
        });
        render_picker_rows(
            self.theme,
            state,
            list_rect,
            content_width,
            self.show_scrollbar,
            buf,
        );

        if let Some(preview_area) = preview_rect {
            render_preview(self.theme, state, preview_area, buf);
        }

        render_footer(self.theme, has_secondary, vertical[3], buf);
    }
}

struct PickerRowHeights {
    len: usize,
    revision: u64,
}

impl ItemHeights for PickerRowHeights {
    fn len(&self) -> usize {
        self.len
    }

    fn height(&self, _index: usize, _width: u16) -> u16 {
        1
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}

fn render_picker_rows<T: PickerItem>(
    theme: &Theme,
    state: &mut PickerState<T>,
    area: Rect,
    content_width: u16,
    show_scrollbar: bool,
    buf: &mut Buffer,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let (list, rows) = state.list_and_rows();
    let content_height = list.layout().content_height;
    let scroll_offset = list.scroll().offset();
    let cache = list.height_cache();
    let visible = visible_rows(cache, scroll_offset, area.height);
    let selected = list.selected();
    let base_style = fg(theme);

    let mut y = area.y;
    let mut remaining = area.height;
    for index in visible.first_index..=visible.last_index {
        if remaining == 0 {
            break;
        }
        let item_h = cache.item_height(index);
        let skip = if index == visible.first_index {
            visible.skip_top
        } else {
            0
        };
        let drawable = (item_h - skip).min(remaining);
        if drawable == 0 {
            continue;
        }
        let row_area = Rect {
            x: area.x,
            y,
            width: content_width,
            height: drawable,
        };
        if selected == Some(index) {
            buf.set_style(row_area, selected_row_style(theme));
        } else {
            buf.set_style(row_area, base_style);
        }
        if skip == 0 {
            render_picker_row(theme, rows, index, selected == Some(index), row_area, buf);
        }
        y += drawable;
        remaining -= drawable;
    }

    if show_scrollbar && area.width > 0 {
        let track = Rect {
            x: area.x + content_width,
            y: area.y,
            width: 1,
            height: area.height,
        };
        let mut scrollbar_state = ScrollbarState::new(content_height.max(1))
            .position(scroll_offset)
            .viewport_content_length(area.height as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        StatefulWidget::render(
            scrollbar,
            track.inner(Margin {
                vertical: 0,
                horizontal: 0,
            }),
            buf,
            &mut scrollbar_state,
        );
    }
}

fn render_picker_row(
    theme: &Theme,
    rows: &[ratakit_core::PickerRow],
    index: usize,
    selected: bool,
    area: Rect,
    buf: &mut Buffer,
) {
    let Some(row) = rows.get(index) else {
        return;
    };
    let base = if selected {
        cursor_style(theme)
    } else {
        fg(theme)
    };
    let line = match &row.kind {
        PickerRowKind::SectionHeader { .. } => Line::from(Span::styled(
            row.text.clone(),
            muted(theme).add_modifier(Modifier::BOLD),
        )),
        PickerRowKind::Item { .. } => Line::from(highlighted_spans(
            &row.text,
            &row.match_indices,
            base,
            accent(theme),
        )),
    };
    Paragraph::new(line).render(area, buf);
}

fn render_query_line(theme: &Theme, query: &str, area: Rect, buf: &mut Buffer) {
    let prompt = Span::styled("> ", accent(theme));
    let text = Span::styled(query, fg(theme));
    Paragraph::new(Line::from(vec![prompt, text])).render(area, buf);
}

fn render_secondary_axis(
    theme: &Theme,
    secondary: &PickerSecondaryState,
    area: Rect,
    buf: &mut Buffer,
) {
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::styled(
        format!("{}: ", secondary.axis().label),
        muted(theme),
    ));
    for (index, option) in secondary.axis().options.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
        }
        let style = if index == secondary.selected() {
            cursor_style(theme)
        } else {
            fg(theme)
        };
        spans.push(Span::styled(option.clone(), style));
    }
    Paragraph::new(Line::from(spans))
        .alignment(Alignment::Left)
        .render(area, buf);
}

fn render_preview<T: PickerItem>(
    theme: &Theme,
    state: &PickerState<T>,
    area: Rect,
    buf: &mut Buffer,
) {
    buf.set_style(area, palette_bg(theme).patch(border(theme)));
    let detail = state
        .selected_detail()
        .map(|d| d.into_owned())
        .unwrap_or_default();
    Paragraph::new(detail).style(muted(theme)).render(area, buf);
}

fn render_footer(theme: &Theme, has_secondary: bool, area: Rect, buf: &mut Buffer) {
    let hints = if has_secondary {
        "↑↓ select · ←→ effort · enter confirm · esc cancel"
    } else {
        "↑↓ select · enter confirm · esc cancel"
    };
    Paragraph::new(Line::from(Span::styled(hints, muted(theme))))
        .alignment(Alignment::Center)
        .render(area, buf);
}

fn highlighted_spans(
    label: &str,
    indices: &[u32],
    base: Style,
    highlight: Style,
) -> Vec<Span<'static>> {
    if indices.is_empty() {
        return vec![Span::styled(label.to_string(), base)];
    }
    let chars: Vec<char> = label.chars().collect();
    let mut spans = Vec::new();
    for (i, ch) in chars.iter().enumerate() {
        let is_match = indices.binary_search(&(i as u32)).is_ok();
        let style = if is_match { highlight } else { base };
        spans.push(Span::styled(ch.to_string(), style));
    }
    spans
}

fn dim_backdrop(area: Rect, buf: &mut Buffer) {
    let dim = Style::default().bg(ratatui_core::style::Color::Rgb(0, 0, 0));
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.set_bg(dim.bg.unwrap_or(ratatui_core::style::Color::Black));
        }
    }
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
