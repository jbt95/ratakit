//! Transcript [`StatefulWidget`] over [`VirtualList`].

use ratakit_core::{
    Component, Event, ItemHeights, Keymap, ListState, Outcome, Theme, VisibleRows, visible_rows,
};
use ratakit_text::Highlighter;
use ratatui_core::style::Modifier;

use crate::widgets::style::fg;
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Margin, Rect};
use ratatui_core::widgets::StatefulWidget;
use ratatui_widgets::scrollbar::{Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::state::{Cell, TranscriptState};
use crate::tool_call_aux::ToolCallCardAux;
use crate::widgets::cells::{measure_cell, render_cell, warm_markdown_layout};

/// Agent transcript: virtualized cells with follow-tail scrolling.
pub struct Transcript<'a> {
    theme: &'a Theme,
    highlighter: &'a dyn Highlighter,
    show_scrollbar: bool,
}

impl<'a> Transcript<'a> {
    /// Create a transcript view (tool rows use `ToolCallCard`, plan rows use `PlanView` helpers).
    pub fn new(theme: &'a Theme, highlighter: &'a dyn Highlighter) -> Self {
        Self {
            theme,
            highlighter,
            show_scrollbar: true,
        }
    }

    /// Show a vertical scrollbar (default: on).
    pub fn scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }
}

impl StatefulWidget for Transcript<'_> {
    type State = TranscriptState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TranscriptState) {
        if area.is_empty() {
            return;
        }

        let content_width = if self.show_scrollbar && area.width > 1 {
            area.width - 1
        } else {
            area.width
        };

        let layout_key = layout_revision(state.revision(), content_width, self.theme);
        let (cells, list, tool_aux) = state.layout_borrow_mut();

        warm_markdown_layout(cells, content_width, self.theme, self.highlighter);

        let heights: Vec<u16> = cells
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                let aux = tool_aux_mut(cell, tool_aux, index);
                measure_cell(cell, content_width, self.theme, self.highlighter, aux)
            })
            .collect();

        let measured = MeasuredHeights {
            heights: &heights,
            len: cells.len(),
            layout_key,
        };

        list.prepare_layout(&measured, content_width, area.height);
        paint_viewport(
            cells,
            list,
            area,
            buf,
            content_width,
            self.theme,
            self.highlighter,
            tool_aux,
            self.show_scrollbar,
        );
    }
}

fn layout_revision(transcript_revision: u64, width: u16, theme: &Theme) -> u64 {
    transcript_revision ^ (u64::from(width) << 32) ^ theme.theme_revision()
}

struct MeasuredHeights<'a> {
    heights: &'a [u16],
    len: usize,
    layout_key: u64,
}

impl ItemHeights for MeasuredHeights<'_> {
    fn len(&self) -> usize {
        self.len
    }

    fn height(&self, index: usize, _width: u16) -> u16 {
        self.heights.get(index).copied().unwrap_or(1).max(1)
    }

    fn revision(&self) -> u64 {
        self.layout_key
    }
}

fn tool_aux_mut<'a>(
    cell: &Cell,
    tool_aux: &'a mut [ToolCallCardAux],
    index: usize,
) -> Option<&'a mut ToolCallCardAux> {
    match cell {
        Cell::ToolCall(_) => tool_aux.get_mut(index),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_viewport(
    cells: &[Cell],
    list: &mut ListState,
    area: Rect,
    buf: &mut Buffer,
    content_width: u16,
    theme: &Theme,
    highlighter: &dyn Highlighter,
    tool_aux: &mut [ToolCallCardAux],
    show_scrollbar: bool,
) {
    let content_height = list.layout().content_height;
    let scroll_offset = list.scroll().offset();
    let cache = list.height_cache();
    let visible: VisibleRows = visible_rows(cache, scroll_offset, area.height);

    let mut y = area.y;
    let mut remaining = area.height;
    let base_style = fg(theme);

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
        if list.selected() == Some(index) {
            buf.set_style(row_area, fg(theme).add_modifier(Modifier::REVERSED));
        } else {
            buf.set_style(row_area, base_style);
        }
        let cell = cells.get(index);
        if let Some(cell) = cell {
            let aux = tool_aux_mut(cell, tool_aux, index);
            render_cell(cell, row_area, buf, skip, theme, highlighter, aux);
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

impl Component for TranscriptState {
    type Intent = ();

    fn handle(&mut self, ev: &Event, keys: &Keymap) -> Outcome<()> {
        self.list_mut().handle(ev, keys)
    }

    fn revision(&self) -> u64 {
        self.data_revision().max(self.list().revision())
    }
}

impl TranscriptState {
    /// Content revision without list scroll state.
    pub fn data_revision(&self) -> u64 {
        self.revision()
    }
}
