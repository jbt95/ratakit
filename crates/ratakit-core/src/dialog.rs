use crate::component::Component;
use crate::event::{Event, KeyCode, KeyEventKind};
use crate::focus::{FocusId, FocusRing};
use crate::keymap::{Keymap, Scope};
use crate::outcome::Outcome;

/// High-level result from a [`DialogState`] interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogIntent {
    /// The user activated the button at this index (left-to-right).
    Confirmed(usize),
    /// The user dismissed the dialog (Escape or equivalent).
    Dismissed,
}

/// Focus and selection state for a modal dialog (labels live on the widget).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogState {
    open: bool,
    selected: usize,
    button_count: usize,
    revision: u64,
}

impl DialogState {
    /// Creates a closed dialog with the first button selected.
    pub fn new() -> Self {
        Self {
            open: false,
            selected: 0,
            button_count: 0,
            revision: 0,
        }
    }

    /// Whether the dialog is currently shown.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Index of the highlighted action button.
    pub fn selected_button(&self) -> usize {
        self.selected
    }

    /// Number of buttons (set by the widget before handling events).
    pub fn button_count(&self) -> usize {
        self.button_count
    }

    /// Updates the button count when the view model changes.
    pub fn set_button_count(&mut self, count: usize) {
        if count == 0 {
            self.selected = 0;
        } else if self.selected >= count {
            self.selected = count - 1;
        }
        if self.button_count != count {
            self.button_count = count;
            self.bump();
        }
    }

    /// Opens the dialog and resets selection to the first button.
    pub fn open(&mut self) {
        if !self.open {
            self.open = true;
            self.selected = 0;
            self.bump();
        }
    }

    /// Closes the dialog without emitting an intent.
    pub fn close(&mut self) {
        if self.open {
            self.open = false;
            self.bump();
        }
    }

    /// Opens the dialog and pushes a focus trap so keyboard focus stays modal.
    pub fn open_with_trap(&mut self, focus: &mut FocusRing, trap: FocusId) {
        self.open();
        focus.push_trap(trap);
    }

    /// Closes the dialog and pops one focus trap if present.
    pub fn close_with_trap(&mut self, focus: &mut FocusRing) {
        self.close();
        if focus.trap_depth() > 0 {
            focus.pop_trap();
        }
    }

    /// Sets the highlighted button, clamped to [`Self::button_count`].
    pub fn select_button(&mut self, index: usize) {
        if self.button_count == 0 {
            return;
        }
        let next = index.min(self.button_count - 1);
        if next != self.selected {
            self.selected = next;
            self.bump();
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if self.button_count == 0 {
            return;
        }
        let len = self.button_count as isize;
        let current = self.selected as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        self.select_button(next);
    }

    fn bump(&mut self) {
        self.revision += 1;
    }
}

impl Default for DialogState {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for DialogState {
    type Intent = DialogIntent;

    fn handle(&mut self, ev: &Event, keys: &Keymap) -> Outcome<Self::Intent> {
        if !self.open || self.button_count == 0 {
            return Outcome::Ignored;
        }

        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                return Outcome::Consumed;
            }

            if let Some(action) = keys.resolve(&[Scope::Modal], key) {
                if action.0 == "dialog.dismiss" {
                    self.close();
                    return Outcome::Intent(DialogIntent::Dismissed);
                }
                if action.0 == "dialog.confirm" {
                    let idx = self.selected;
                    self.close();
                    return Outcome::Intent(DialogIntent::Confirmed(idx));
                }
            }

            match key.code {
                KeyCode::Escape => {
                    self.close();
                    Outcome::Intent(DialogIntent::Dismissed)
                }
                KeyCode::Enter => {
                    let idx = self.selected;
                    self.close();
                    Outcome::Intent(DialogIntent::Confirmed(idx))
                }
                KeyCode::Left | KeyCode::Up => {
                    self.move_selection(-1);
                    Outcome::Consumed
                }
                KeyCode::Right | KeyCode::Down | KeyCode::Tab => {
                    self.move_selection(1);
                    Outcome::Consumed
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap_or(0) as usize;
                    if digit > 0 && digit <= self.button_count {
                        let idx = digit - 1;
                        self.close();
                        Outcome::Intent(DialogIntent::Confirmed(idx))
                    } else {
                        Outcome::Ignored
                    }
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
