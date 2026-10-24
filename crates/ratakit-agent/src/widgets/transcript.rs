//! Transcript [`StatefulWidget`] over [`VirtualList`].

use ratakit_core::{Component, Event, ItemHeights, Keymap, Outcome, Theme};
use ratakit_text::Highlighter;
use ratakit_widgets::{RowRenderer, VirtualList};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;

use crate::state::TranscriptState;
use crate::widgets::cells::{measure_cell, render_cell, warm_markdown_layout};
use crate::widgets::extras::TranscriptExtras;

/// Agent transcript: virtualized cells with follow-tail scrolling.
pub struct Transcript<'a> {
    theme: &'a Theme,
    highlighter: &'a dyn Highlighter,
    extras: TranscriptExtras<'a>,
    show_scrollbar: bool,
}

impl<'a> Transcript<'a> {
    /// Create a transcript view with default placeholder tool/plan cells.
    pub fn new(theme: &'a Theme, highlighter: &'a dyn Highlighter) -> Self {
        Self {
            theme,
            highlighter,
            extras: TranscriptExtras::default(),
            show_scrollbar: true,
        }
    }

    /// Optional renderers for tool-call and plan cells (A3 / A7).
    pub fn extras(mut self, extras: TranscriptExtras<'a>) -> Self {
        self.extras = extras;
        self
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
        let (cells, list) = state.cells_and_list_mut();

        warm_markdown_layout(cells, content_width, self.theme, self.highlighter);

        let heights: Vec<u16> = cells
            .iter()
            .map(|cell| measure_cell(cell, content_width, self.theme, self.extras))
            .collect();
        let rows = TranscriptRows {
            cells: &*cells,
            heights: &heights,
            theme: self.theme,
            extras: self.extras,
            layout_key,
        };

        list.prepare_layout(&rows, content_width, area.height);
        let list_widget = VirtualList::new(&rows, self.theme).scrollbar(self.show_scrollbar);
        list_widget.render(area, buf, list);
    }
}

fn layout_revision(transcript_revision: u64, width: u16, theme: &Theme) -> u64 {
    transcript_revision ^ (u64::from(width) << 32) ^ theme.theme_revision()
}

struct TranscriptRows<'a> {
    cells: &'a [crate::state::Cell],
    heights: &'a [u16],
    theme: &'a Theme,
    extras: TranscriptExtras<'a>,
    layout_key: u64,
}

impl ItemHeights for TranscriptRows<'_> {
    fn len(&self) -> usize {
        self.cells.len()
    }

    fn height(&self, index: usize, _width: u16) -> u16 {
        self.heights.get(index).copied().unwrap_or(1).max(1)
    }

    fn revision(&self) -> u64 {
        self.layout_key
    }
}

impl RowRenderer for TranscriptRows<'_> {
    fn render_row(&self, index: usize, area: Rect, buf: &mut Buffer, skip_top: u16) {
        let cell = self.cells.get(index);
        if let Some(cell) = cell {
            render_cell(cell, area, buf, skip_top, self.theme, self.extras);
        }
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
