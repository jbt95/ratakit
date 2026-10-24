//! Virtualized vertical list with variable row heights and optional scrollbar.

use ratakit_core::{ItemHeights, ListState, Theme, VisibleRows, visible_rows};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Margin, Rect};
use ratatui_core::widgets::StatefulWidget;
use ratatui_widgets::scrollbar::{Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::style::{fg, selected_row_style};

/// Renders one logical row into the buffer. `skip_top` skips leading rows when the row is
/// partially scrolled out of the viewport.
pub trait RowRenderer: ItemHeights {
    /// Draw item `index` into `area`. Only the bottom `area.height` rows are visible; skip the
    /// first `skip_top` rows of the item's content.
    fn render_row(&self, index: usize, area: Rect, buf: &mut Buffer, skip_top: u16);
}

/// A virtualized list that only paints rows intersecting the viewport.
///
/// Pass a type implementing [`RowRenderer`] for data and drawing. State lives in [`ListState`]
/// (scroll offset, follow-tail, optional selection).
pub struct VirtualList<'a, R> {
    rows: &'a R,
    theme: &'a Theme,
    show_scrollbar: bool,
}

impl<'a, R> VirtualList<'a, R> {
    /// Creates a list without a scrollbar.
    #[must_use]
    pub fn new(rows: &'a R, theme: &'a Theme) -> Self {
        Self {
            rows,
            theme,
            show_scrollbar: false,
        }
    }

    /// Enables or disables the vertical scrollbar (reserves one column on the right).
    #[must_use]
    pub fn scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }
}

impl<'a, R: RowRenderer> StatefulWidget for VirtualList<'a, R> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListState) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let content_width = if self.show_scrollbar && area.width > 1 {
            area.width - 1
        } else {
            area.width
        };

        state.prepare_layout(self.rows, content_width, area.height);
        let content_height = state.layout().content_height;
        let scroll_offset = state.scroll().offset();
        let cache = state.height_cache();
        let visible: VisibleRows = visible_rows(cache, scroll_offset, area.height);

        let mut y = area.y;
        let mut remaining = area.height;
        let base_style = fg(self.theme);

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
            let is_selected = state.selected() == Some(index);
            if is_selected {
                buf.set_style(row_area, selected_row_style(self.theme));
            } else {
                buf.set_style(row_area, base_style);
            }
            self.rows.render_row(index, row_area, buf, skip);
            y += drawable;
            remaining -= drawable;
        }

        if self.show_scrollbar && area.width > 0 {
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
}
