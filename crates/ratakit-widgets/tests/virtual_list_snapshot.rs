use ratakit_core::{ListState, Theme};
use ratakit_testing::assert_stateful_widget_snapshot;
use ratakit_widgets::{RowRenderer, VirtualList};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;
use ratatui_widgets::paragraph::Paragraph;

struct DemoRows {
    labels: Vec<String>,
}

impl DemoRows {
    fn new(labels: &[&str]) -> Self {
        Self {
            labels: labels.iter().map(|s| (*s).to_string()).collect(),
        }
    }
}

impl ratakit_core::ItemHeights for DemoRows {
    fn len(&self) -> usize {
        self.labels.len()
    }

    fn height(&self, index: usize, width: u16) -> u16 {
        let label = &self.labels[index];
        let cols = width.max(1) as usize;
        let lines = label.len().div_ceil(cols).max(1);
        lines as u16
    }
}

impl RowRenderer for DemoRows {
    fn render_row(&self, index: usize, area: Rect, buf: &mut Buffer, skip_top: u16) {
        let text = &self.labels[index];
        let cols = area.width.max(1) as usize;
        let lines: Vec<Line> = text
            .chars()
            .collect::<Vec<_>>()
            .chunks(cols)
            .map(|chunk| Line::from(chunk.iter().collect::<String>()))
            .collect();
        let start = skip_top.min(lines.len() as u16) as usize;
        let visible = lines.into_iter().skip(start).take(area.height as usize);
        Paragraph::new(visible.collect::<Vec<_>>())
            .style(Style::default())
            .render(area, buf);
    }
}

#[test]
fn virtual_list_viewport_40x5() {
    let theme = Theme::dark();
    let rows = DemoRows::new(&[
        "alpha",
        "beta line two",
        "gamma is a longer row for wrapping",
        "delta",
        "epsilon",
        "zeta",
    ]);
    let widget = VirtualList::new(&rows, &theme);
    let mut state = ListState::new();
    assert_stateful_widget_snapshot!("virtual_list_40x5", widget, 40, 5, &mut state);
}

#[test]
fn virtual_list_with_scrollbar_30x4() {
    let theme = Theme::dark();
    let rows = DemoRows::new(&["one", "two", "three", "four", "five", "six", "seven"]);
    let widget = VirtualList::new(&rows, &theme).scrollbar(true);
    let mut state = ListState::new();
    state.prepare_layout(&rows, 29, 4);
    let content_height = state.layout().content_height;
    state.scroll_mut().scroll_by(-2, 4, content_height);
    assert_stateful_widget_snapshot!("virtual_list_scrollbar_30x4", widget, 30, 4, &mut state);
}
