use crate::component::Component;
use crate::event::{Event, KeyCode, KeyEventKind};
use crate::keymap::{Keymap, Scope};
use crate::outcome::Outcome;

/// Sort order for [`TableState::sort_column`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

/// Row/column selection, scroll offset, and sort metadata for a data table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableState {
    row_count: usize,
    column_count: usize,
    offset: usize,
    selected: Option<usize>,
    sort_column: Option<usize>,
    sort_direction: SortDirection,
    cached_width: u16,
    cached_column_widths: Vec<u16>,
    revision: u64,
}

impl TableState {
    pub fn new() -> Self {
        Self {
            row_count: 0,
            column_count: 0,
            offset: 0,
            selected: None,
            sort_column: None,
            sort_direction: SortDirection::Ascending,
            cached_width: 0,
            cached_column_widths: Vec::new(),
            revision: 0,
        }
    }

    pub fn set_dimensions(&mut self, rows: usize, columns: usize) {
        let changed = self.row_count != rows || self.column_count != columns;
        self.row_count = rows;
        self.column_count = columns;
        if self.selected.is_some_and(|r| r >= rows) {
            self.selected = if rows == 0 { None } else { Some(rows - 1) };
        }
        if changed {
            self.cached_column_widths.clear();
            self.cached_width = 0;
            self.bump();
        }
    }

    pub fn row_count(&self) -> usize {
        self.row_count
    }

    pub fn column_count(&self) -> usize {
        self.column_count
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn selected_row(&self) -> Option<usize> {
        self.selected
    }

    pub fn sort_column(&self) -> Option<usize> {
        self.sort_column
    }

    pub fn sort_direction(&self) -> SortDirection {
        self.sort_direction
    }

    pub fn cached_column_widths(&self) -> &[u16] {
        &self.cached_column_widths
    }

    /// Toggles or sets sort on `column` (same column flips direction).
    pub fn toggle_sort(&mut self, column: usize) {
        if self.column_count == 0 || column >= self.column_count {
            return;
        }
        if self.sort_column == Some(column) {
            self.sort_direction = match self.sort_direction {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.sort_column = Some(column);
            self.sort_direction = SortDirection::Ascending;
        }
        self.bump();
    }

    pub fn select_row(&mut self, row: Option<usize>) {
        if row.is_none_or(|r| r < self.row_count) && self.selected != row {
            self.selected = row;
            self.bump();
        }
    }

    pub fn select_next_row(&mut self) {
        if self.row_count == 0 {
            return;
        }
        let next = match self.selected {
            None => 0,
            Some(i) => (i + 1).min(self.row_count - 1),
        };
        self.select_row(Some(next));
    }

    pub fn select_prev_row(&mut self) {
        if self.row_count == 0 {
            return;
        }
        let next = match self.selected {
            None => 0,
            Some(i) => i.saturating_sub(1),
        };
        self.select_row(Some(next));
    }

    /// Keeps `selected` visible in a viewport of `visible_rows` body lines.
    pub fn ensure_visible(&mut self, visible_rows: usize) {
        if visible_rows == 0 {
            return;
        }
        let row = self.selected.unwrap_or(0);
        if row < self.offset {
            self.offset = row;
            self.bump();
        } else if row >= self.offset + visible_rows {
            self.offset = row + 1 - visible_rows;
            self.bump();
        }
    }

    pub fn scroll_by(&mut self, delta: isize, visible_rows: usize) {
        if self.row_count == 0 || visible_rows == 0 {
            return;
        }
        let max_offset = self.row_count.saturating_sub(visible_rows);
        let next = (self.offset as isize + delta).clamp(0, max_offset as isize) as usize;
        if next != self.offset {
            self.offset = next;
            self.bump();
        }
    }

    /// Stores auto-sized column widths for `(width, revision)` invalidation in the widget.
    pub fn set_column_width_cache(&mut self, total_width: u16, widths: Vec<u16>) {
        if self.cached_width != total_width || self.cached_column_widths != widths {
            self.cached_width = total_width;
            self.cached_column_widths = widths;
            self.bump();
        }
    }

    fn bump(&mut self) {
        self.revision += 1;
    }
}

impl Default for TableState {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for TableState {
    type Intent = ();

    fn handle(&mut self, ev: &Event, keys: &Keymap) -> Outcome<Self::Intent> {
        if self.row_count == 0 {
            return Outcome::Ignored;
        }

        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                return Outcome::Consumed;
            }

            if let Some(action) = keys.resolve(&[Scope::Focused, Scope::Global], key) {
                match action.0.as_str() {
                    "table.next" => {
                        self.select_next_row();
                        return Outcome::Consumed;
                    }
                    "table.prev" => {
                        self.select_prev_row();
                        return Outcome::Consumed;
                    }
                    _ => {}
                }
            }

            match key.code {
                KeyCode::Down => {
                    self.select_next_row();
                    Outcome::Consumed
                }
                KeyCode::Up => {
                    self.select_prev_row();
                    Outcome::Consumed
                }
                KeyCode::PageDown => {
                    self.scroll_by(5, 5);
                    Outcome::Consumed
                }
                KeyCode::PageUp => {
                    self.scroll_by(-5, 5);
                    Outcome::Consumed
                }
                KeyCode::Home => {
                    self.select_row(Some(0));
                    Outcome::Consumed
                }
                KeyCode::End => {
                    self.select_row(Some(self.row_count - 1));
                    Outcome::Consumed
                }
                _ => Outcome::Ignored,
            }
        } else {
            Outcome::Ignored
        }
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}
