use crate::component::Component;
use crate::event::{Event, KeyCode, KeyEventKind};
use crate::keymap::{Keymap, Scope};
use crate::outcome::Outcome;

/// Result of changing a checkbox or switch value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleIntent {
    Changed(bool),
}

/// Shared checked/disabled state for [`CheckboxState`] and [`SwitchState`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ToggleState {
    checked: bool,
    disabled: bool,
    revision: u64,
}

impl ToggleState {
    pub fn new(checked: bool) -> Self {
        Self {
            checked,
            disabled: false,
            revision: 0,
        }
    }

    pub fn is_checked(&self) -> bool {
        self.checked
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn set_checked(&mut self, checked: bool) {
        if self.checked != checked {
            self.checked = checked;
            self.bump();
        }
    }

    pub fn set_disabled(&mut self, disabled: bool) {
        if self.disabled != disabled {
            self.disabled = disabled;
            self.bump();
        }
    }

    /// Toggle the value unless this control is disabled.
    pub fn toggle(&mut self) -> Option<bool> {
        if self.disabled {
            return None;
        }
        self.checked = !self.checked;
        self.bump();
        Some(self.checked)
    }

    fn bump(&mut self) {
        self.revision += 1;
    }
}

impl Component for ToggleState {
    type Intent = ToggleIntent;

    fn handle(&mut self, event: &Event, keys: &Keymap) -> Outcome<Self::Intent> {
        let Event::Key(key) = event else {
            return Outcome::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return Outcome::Consumed;
        }
        if self.disabled {
            return Outcome::Ignored;
        }

        let activate = keys
            .resolve(&[Scope::Focused, Scope::Global], key)
            .is_some_and(|action| action.0 == "toggle.activate");
        if activate || matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
            return self.toggle().map_or(Outcome::Ignored, |checked| {
                Outcome::Intent(ToggleIntent::Changed(checked))
            });
        }
        Outcome::Ignored
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}

impl ToggleState {
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

/// Checkbox state; an alias of [`ToggleState`] so state semantics stay identical.
pub type CheckboxState = ToggleState;

/// Switch state; an alias of [`ToggleState`] so state semantics stay identical.
pub type SwitchState = ToggleState;

/// Result of operating a radio group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioGroupIntent {
    Selected(usize),
}

/// Selected option and option count for a radio group.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RadioGroupState {
    selected: usize,
    option_count: usize,
    revision: u64,
}

impl RadioGroupState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn option_count(&self) -> usize {
        self.option_count
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn set_option_count(&mut self, count: usize) {
        let selected = if count == 0 {
            0
        } else {
            self.selected.min(count - 1)
        };
        if self.option_count != count || self.selected != selected {
            self.option_count = count;
            self.selected = selected;
            self.bump();
        }
    }

    pub fn select(&mut self, index: usize) -> Option<usize> {
        if self.option_count == 0 {
            return None;
        }
        let selected = index.min(self.option_count - 1);
        if self.selected != selected {
            self.selected = selected;
            self.bump();
        }
        Some(self.selected)
    }

    pub fn select_next(&mut self) -> Option<usize> {
        if self.option_count == 0 {
            return None;
        }
        self.select((self.selected + 1) % self.option_count)
    }

    pub fn select_previous(&mut self) -> Option<usize> {
        if self.option_count == 0 {
            return None;
        }
        self.select((self.selected + self.option_count - 1) % self.option_count)
    }

    fn bump(&mut self) {
        self.revision += 1;
    }
}

impl Component for RadioGroupState {
    type Intent = RadioGroupIntent;

    fn handle(&mut self, event: &Event, keys: &Keymap) -> Outcome<Self::Intent> {
        if self.option_count == 0 {
            return Outcome::Ignored;
        }
        let Event::Key(key) = event else {
            return Outcome::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return Outcome::Consumed;
        }

        let action = keys
            .resolve(&[Scope::Focused, Scope::Global], key)
            .map(|action| action.0.as_str());
        let selected = match action {
            Some("radio.next") => self.select_next(),
            Some("radio.previous") => self.select_previous(),
            Some("radio.first") => self.select(0),
            Some("radio.last") => self.select(self.option_count - 1),
            _ => match key.code {
                KeyCode::Down | KeyCode::Right | KeyCode::Char('j') => self.select_next(),
                KeyCode::Up | KeyCode::Left | KeyCode::Char('k') => self.select_previous(),
                KeyCode::Home => self.select(0),
                KeyCode::End => self.select(self.option_count - 1),
                _ => return Outcome::Ignored,
            },
        };
        selected.map_or(Outcome::Ignored, |index| {
            Outcome::Intent(RadioGroupIntent::Selected(index))
        })
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}

/// Result of operating a select control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectIntent {
    Opened,
    Closed,
    Selected(usize),
}

/// Open, highlighted, and committed selection state for the `Select` widget.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectState {
    option_count: usize,
    selected: Option<usize>,
    highlighted: Option<usize>,
    open: bool,
    revision: u64,
}

impl SelectState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn option_count(&self) -> usize {
        self.option_count
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn highlighted(&self) -> Option<usize> {
        self.highlighted
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn set_option_count(&mut self, count: usize) {
        let selected = self.selected.filter(|index| *index < count);
        let highlighted = self.highlighted.filter(|index| *index < count).or(selected);
        if self.option_count != count
            || self.selected != selected
            || self.highlighted != highlighted
        {
            self.option_count = count;
            self.selected = selected;
            self.highlighted = highlighted;
            if count == 0 {
                self.open = false;
            }
            self.bump();
        }
    }

    pub fn select(&mut self, index: usize) -> Option<usize> {
        if self.option_count == 0 {
            return None;
        }
        let selected = index.min(self.option_count - 1);
        if self.selected != Some(selected) || self.highlighted != Some(selected) {
            self.selected = Some(selected);
            self.highlighted = Some(selected);
            self.bump();
        }
        Some(selected)
    }

    pub fn open(&mut self) -> bool {
        if self.open || self.option_count == 0 {
            return false;
        }
        self.open = true;
        self.highlighted = Some(self.selected.unwrap_or(0));
        self.bump();
        true
    }

    pub fn close(&mut self) -> bool {
        if !self.open {
            return false;
        }
        self.open = false;
        self.highlighted = self.selected;
        self.bump();
        true
    }

    fn move_highlight(&mut self, forward: bool) {
        if self.option_count == 0 {
            return;
        }
        let current = self.highlighted.unwrap_or(0);
        let next = if forward {
            (current + 1) % self.option_count
        } else {
            (current + self.option_count - 1) % self.option_count
        };
        if self.highlighted != Some(next) {
            self.highlighted = Some(next);
            self.bump();
        }
    }

    fn confirm(&mut self) -> Option<usize> {
        let highlighted = self.highlighted?;
        self.selected = Some(highlighted);
        self.open = false;
        self.bump();
        Some(highlighted)
    }

    fn bump(&mut self) {
        self.revision += 1;
    }
}

impl Component for SelectState {
    type Intent = SelectIntent;

    fn handle(&mut self, event: &Event, keys: &Keymap) -> Outcome<Self::Intent> {
        if self.option_count == 0 {
            return Outcome::Ignored;
        }
        let Event::Key(key) = event else {
            return Outcome::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return Outcome::Consumed;
        }

        let action = keys
            .resolve(&[Scope::Focused, Scope::Global], key)
            .map(|action| action.0.as_str());
        if !self.open {
            let should_open = matches!(action, Some("select.open"))
                || matches!(
                    key.code,
                    KeyCode::Enter | KeyCode::Down | KeyCode::Char(' ')
                );
            return if should_open && self.open() {
                Outcome::Intent(SelectIntent::Opened)
            } else {
                Outcome::Ignored
            };
        }

        match action {
            Some("select.close") => {
                self.close();
                return Outcome::Intent(SelectIntent::Closed);
            }
            Some("select.next") => {
                self.move_highlight(true);
                return Outcome::Consumed;
            }
            Some("select.previous") => {
                self.move_highlight(false);
                return Outcome::Consumed;
            }
            Some("select.confirm") => {
                return self.confirm().map_or(Outcome::Ignored, |index| {
                    Outcome::Intent(SelectIntent::Selected(index))
                });
            }
            _ => {}
        }

        match key.code {
            KeyCode::Escape => {
                self.close();
                Outcome::Intent(SelectIntent::Closed)
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.confirm().map_or(Outcome::Ignored, |index| {
                    Outcome::Intent(SelectIntent::Selected(index))
                })
            }
            KeyCode::Down | KeyCode::Right => {
                self.move_highlight(true);
                Outcome::Consumed
            }
            KeyCode::Up | KeyCode::Left => {
                self.move_highlight(false);
                Outcome::Consumed
            }
            KeyCode::Home => {
                self.highlighted = Some(0);
                self.bump();
                Outcome::Consumed
            }
            KeyCode::End => {
                self.highlighted = Some(self.option_count - 1);
                self.bump();
                Outcome::Consumed
            }
            _ => Outcome::Ignored,
        }
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}

#[cfg(test)]
mod tests {
    use crate::{Component, Event, KeyCode, KeyEvent, Keymap, Outcome};

    use super::{
        RadioGroupIntent, RadioGroupState, SelectIntent, SelectState, ToggleIntent, ToggleState,
    };

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code))
    }

    #[test]
    fn toggle_state_changes_and_respects_disabled_state() {
        let mut state = ToggleState::new(false);
        let keys = Keymap::empty();
        assert_eq!(
            state.handle(&key(KeyCode::Char(' ')), &keys),
            Outcome::Intent(ToggleIntent::Changed(true))
        );
        state.set_disabled(true);
        assert_eq!(state.handle(&key(KeyCode::Enter), &keys), Outcome::Ignored);
        assert!(state.is_checked());
    }

    #[test]
    fn radio_group_wraps_and_clamps_selection() {
        let mut state = RadioGroupState::new();
        state.set_option_count(3);
        let keys = Keymap::empty();
        assert_eq!(
            state.handle(&key(KeyCode::Up), &keys),
            Outcome::Intent(RadioGroupIntent::Selected(2))
        );
        state.set_option_count(1);
        assert_eq!(state.selected(), 0);
    }

    #[test]
    fn select_commits_highlight_and_escape_keeps_previous_value() {
        let mut state = SelectState::new();
        state.set_option_count(3);
        state.select(0);
        let keys = Keymap::empty();
        assert_eq!(
            state.handle(&key(KeyCode::Enter), &keys),
            Outcome::Intent(SelectIntent::Opened)
        );
        state.handle(&key(KeyCode::Down), &keys);
        assert_eq!(state.selected(), Some(0));
        assert_eq!(
            state.handle(&key(KeyCode::Enter), &keys),
            Outcome::Intent(SelectIntent::Selected(1))
        );
        state.handle(&key(KeyCode::Enter), &keys);
        state.handle(&key(KeyCode::Down), &keys);
        assert_eq!(
            state.handle(&key(KeyCode::Escape), &keys),
            Outcome::Intent(SelectIntent::Closed)
        );
        assert_eq!(state.selected(), Some(1));
    }
}
