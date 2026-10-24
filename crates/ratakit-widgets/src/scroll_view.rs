//! Scrollable viewport for content taller than the allocated area.

use ratakit_core::{ScrollState, Theme};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Margin, Rect};
use ratatui_core::widgets::StatefulWidget;
use ratatui_widgets::scrollbar::{Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::style::fg;

/// How [`ScrollView`] maps content height to the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollViewMode {
    /// Content height is at least the viewport height so the area is filled (default).
    #[default]
    Fill,
    /// When content is shorter than the area, only the content height scrolls.
    Fit,
}

/// Renders arbitrary content with a vertical scroll offset.
pub trait ScrollViewContent {
    /// Total content height in terminal rows at `width`.
    fn content_height(&self, width: u16) -> usize;

    /// Draw the slice of content visible in `area`, starting at `y_offset` rows from the top.
    fn render(&self, area: Rect, buf: &mut Buffer, y_offset: usize);
}

/// A vertically scrollable viewport over [`ScrollViewContent`].
pub struct ScrollView<'a, C> {
    content: &'a C,
    theme: &'a Theme,
    mode: ScrollViewMode,
    show_scrollbar: bool,
}

impl<'a, C> ScrollView<'a, C> {
    #[must_use]
    pub fn new(content: &'a C, theme: &'a Theme) -> Self {
        Self {
            content,
            theme,
            mode: ScrollViewMode::default(),
            show_scrollbar: false,
        }
    }

    #[must_use]
    pub fn mode(mut self, mode: ScrollViewMode) -> Self {
        self.mode = mode;
        self
    }

    #[must_use]
    pub fn scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }
}

impl<C: ScrollViewContent> StatefulWidget for ScrollView<'_, C> {
    type State = ScrollState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ScrollState) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let content_width = if self.show_scrollbar && area.width > 1 {
            area.width - 1
        } else {
            area.width
        };

        let measured = self.content.content_height(content_width);
        let content_height = match self.mode {
            ScrollViewMode::Fill => measured.max(area.height as usize),
            ScrollViewMode::Fit => measured,
        };

        state.sync_layout(area.height, content_height);
        let scroll_offset = state.offset();

        let view_area = Rect {
            x: area.x,
            y: area.y,
            width: content_width,
            height: area.height,
        };
        buf.set_style(view_area, fg(self.theme));
        self.content.render(view_area, buf, scroll_offset);

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
