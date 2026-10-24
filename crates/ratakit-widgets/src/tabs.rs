use ratakit_core::{TabsState, Theme};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Modifier;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::tabs::Tabs as RtTabs;

use crate::style::{accent, cursor_style, fg, muted};

/// Metadata for one tab title (badge count and activity dot).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabItem {
    pub label: String,
    pub badge: Option<u32>,
    pub active: bool,
}

impl TabItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            badge: None,
            active: false,
        }
    }

    pub fn with_badge(mut self, count: u32) -> Self {
        self.badge = Some(count);
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

/// Tab strip with optional badges and activity indicators.
pub struct Tabs<'a> {
    theme: &'a Theme,
    items: &'a [TabItem],
}

impl<'a> Tabs<'a> {
    pub fn new(theme: &'a Theme, items: &'a [TabItem]) -> Self {
        Self { theme, items }
    }

    fn title_line(&self, item: &TabItem, selected: bool) -> Line<'static> {
        let mut parts: Vec<Span<'static>> = Vec::new();
        if item.active {
            parts.push(Span::styled("● ", accent(self.theme)));
        }
        let base = if selected {
            cursor_style(self.theme)
        } else {
            fg(self.theme)
        };
        parts.push(Span::styled(item.label.clone(), base));
        if let Some(n) = item.badge {
            let badge = format!(" {n}");
            parts.push(Span::styled(
                badge,
                muted(self.theme).add_modifier(Modifier::DIM),
            ));
        }
        Line::from(parts)
    }
}

impl StatefulWidget for Tabs<'_> {
    type State = TabsState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.set_tab_count(self.items.len());
        if self.items.is_empty() {
            return;
        }

        let titles: Vec<Line<'static>> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| self.title_line(item, i == state.selected()))
            .collect();

        let widget = RtTabs::new(titles)
            .select(state.selected())
            .style(fg(self.theme))
            .highlight_style(cursor_style(self.theme))
            .divider(Span::styled(" │ ", muted(self.theme)));

        Widget::render(widget, area, buf);
    }
}
