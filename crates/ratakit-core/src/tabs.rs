use crate::component::Component;
use crate::event::{Event, KeyCode, KeyEventKind};
use crate::keymap::{Keymap, Scope};
use crate::outcome::Outcome;

/// Selection state for a tab strip (titles and badges are supplied by the widget).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabsState {
    selected: usize,
    tab_count: usize,
    revision: u64,
}

impl TabsState {
    /// Creates state with the first tab selected.
    pub fn new() -> Self {
        Self {
            selected: 0,
            tab_count: 0,
            revision: 0,
        }
    }

    /// Updates the number of tabs (call when the tab list changes).
    pub fn set_tab_count(&mut self, count: usize) {
        if count == 0 {
            self.selected = 0;
        } else if self.selected >= count {
            self.selected = count - 1;
        }
        if self.tab_count != count {
            self.tab_count = count;
            self.bump();
        }
    }

    pub fn tab_count(&self) -> usize {
        self.tab_count
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn select(&mut self, index: usize) {
        if self.tab_count == 0 {
            return;
        }
        let next = index.min(self.tab_count - 1);
        if next != self.selected {
            self.selected = next;
            self.bump();
        }
    }

    pub fn select_next(&mut self) {
        if self.tab_count == 0 {
            return;
        }
        let next = (self.selected + 1) % self.tab_count;
        if next != self.selected {
            self.selected = next;
            self.bump();
        }
    }

    pub fn select_prev(&mut self) {
        if self.tab_count == 0 {
            return;
        }
        let next = (self.selected + self.tab_count - 1) % self.tab_count;
        if next != self.selected {
            self.selected = next;
            self.bump();
        }
    }

    fn bump(&mut self) {
        self.revision += 1;
    }
}

impl Default for TabsState {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for TabsState {
    type Intent = usize;

    fn handle(&mut self, ev: &Event, keys: &Keymap) -> Outcome<Self::Intent> {
        if self.tab_count == 0 {
            return Outcome::Ignored;
        }

        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                return Outcome::Consumed;
            }

            if let Some(action) = keys.resolve(&[Scope::Focused, Scope::Global], key) {
                match action.0.as_str() {
                    "tabs.next" => {
                        self.select_next();
                        return Outcome::Intent(self.selected);
                    }
                    "tabs.prev" => {
                        self.select_prev();
                        return Outcome::Intent(self.selected);
                    }
                    _ => {}
                }
            }

            match key.code {
                KeyCode::Right | KeyCode::Tab => {
                    self.select_next();
                    Outcome::Intent(self.selected)
                }
                KeyCode::Left => {
                    self.select_prev();
                    Outcome::Intent(self.selected)
                }
                KeyCode::Home => {
                    self.select(0);
                    Outcome::Intent(self.selected)
                }
                KeyCode::End => {
                    self.select(self.tab_count - 1);
                    Outcome::Intent(self.selected)
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
