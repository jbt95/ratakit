use ratakit_core::{SortDirection, TableState, Theme};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Rect};
use ratatui_core::style::Modifier;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::StatefulWidget;
use ratatui_widgets::table::{Row, Table as RtTable, TableState as RtTableState};
use unicode_width::UnicodeWidthStr;

use crate::style::{accent, cursor_style, fg};

/// One column header and body cells as plain text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableColumn {
    pub header: String,
    pub cells: Vec<String>,
}

impl TableColumn {
    pub fn new(header: impl Into<String>, cells: Vec<String>) -> Self {
        Self {
            header: header.into(),
            cells,
        }
    }
}

/// Virtualized, sortable table with auto-sized columns.
pub struct Table<'a> {
    theme: &'a Theme,
    columns: &'a [TableColumn],
}

impl<'a> Table<'a> {
    pub fn new(theme: &'a Theme, columns: &'a [TableColumn]) -> Self {
        Self { theme, columns }
    }

    fn row_count(&self) -> usize {
        self.columns.first().map_or(0, |c| c.cells.len())
    }

    fn header_line(&self, state: &TableState, col: usize) -> Line<'static> {
        let header = self.columns.get(col).map_or("", |c| c.header.as_str());
        let mut label = header.to_string();
        if state.sort_column() == Some(col) {
            let arrow = match state.sort_direction() {
                SortDirection::Ascending => " ▲",
                SortDirection::Descending => " ▼",
            };
            label.push_str(arrow);
        }
        let style = if state.sort_column() == Some(col) {
            accent(self.theme)
        } else {
            fg(self.theme).add_modifier(Modifier::BOLD)
        };
        Line::from(Span::styled(label, style))
    }

    fn compute_widths(&self, total_width: u16, state: &TableState) -> Vec<Constraint> {
        let cols = self.columns.len();
        if cols == 0 || total_width == 0 {
            return Vec::new();
        }

        if !state.cached_column_widths().is_empty() && state.cached_column_widths().len() == cols {
            return state
                .cached_column_widths()
                .iter()
                .map(|w| Constraint::Length(*w))
                .collect();
        }

        let mut widths: Vec<u16> = Vec::with_capacity(cols);
        for (i, col) in self.columns.iter().enumerate() {
            let mut max_w = UnicodeWidthStr::width(self.header_line(state, i).to_string().as_str());
            for row in &col.cells {
                max_w = max_w.max(UnicodeWidthStr::width(row.as_str()));
            }
            widths.push((max_w as u16 + 2).min(total_width));
        }

        let sum = widths.iter().map(|w| *w as u32).sum::<u32>();
        if sum > total_width as u32 && cols > 0 {
            let scale = total_width as f32 / sum as f32;
            for w in &mut widths {
                *w = ((*w as f32) * scale).max(3.0) as u16;
            }
        }

        widths.iter().map(|w| Constraint::Length(*w)).collect()
    }
}

impl StatefulWidget for Table<'_> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let rows = self.row_count();
        let cols = self.columns.len();
        state.set_dimensions(rows, cols);
        if cols == 0 || area.height < 2 {
            return;
        }

        let body_height = area.height.saturating_sub(1) as usize;
        state.ensure_visible(body_height);

        let widths = self.compute_widths(area.width, state);
        if widths.is_empty() {
            return;
        }

        let width_values: Vec<u16> = widths
            .iter()
            .filter_map(|c| match c {
                Constraint::Length(n) => Some(*n),
                _ => None,
            })
            .collect();
        state.set_column_width_cache(area.width, width_values);

        let header = Row::new(
            (0..cols)
                .map(|i| self.header_line(state, i))
                .collect::<Vec<_>>(),
        )
        .style(fg(self.theme).add_modifier(Modifier::BOLD))
        .bottom_margin(0);

        let start = state.offset();
        let end = (start + body_height).min(rows);
        let data_rows: Vec<Row<'static>> = (start..end)
            .map(|row_idx| {
                let cells: Vec<Line<'static>> = self
                    .columns
                    .iter()
                    .map(|col| {
                        let text = col.cells.get(row_idx).map_or("", |s| s.as_str());
                        Line::from(Span::styled(text.to_string(), fg(self.theme)))
                    })
                    .collect();
                Row::new(cells)
            })
            .collect();

        let rt_state = RtTableState::default()
            .with_offset(0)
            .with_selected(state.selected_row());

        let table = RtTable::new(data_rows, widths.clone())
            .header(header)
            .row_highlight_style(cursor_style(self.theme))
            .column_spacing(1);

        let mut rt = rt_state;
        if let Some(sel) = state.selected_row() {
            let visible = sel.saturating_sub(start);
            rt.select(Some(visible));
        }
        StatefulWidget::render(table, area, buf, &mut rt);
    }
}
