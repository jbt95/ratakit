//! Fuzzy picker state (no ratatui). Used by [`FuzzyPicker`](https://docs.rs/ratakit-widgets) and agent pickers.

use std::borrow::Cow;

use crate::component::Component;
use crate::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use crate::focus::{FocusId, FocusRing};
use crate::fuzzy::FuzzyFilter;
use crate::keymap::{Keymap, Scope};
use crate::list::ListState;
use crate::outcome::Outcome;
use crate::revision::Revisioned;

/// One row in a grouped picker (e.g. models under a provider).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerSection<T> {
    pub title: String,
    pub items: Vec<T>,
}

/// Secondary axis rendered above the list (e.g. reasoning effort: low / medium / high).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerSecondaryAxis {
    pub label: String,
    pub options: Vec<String>,
}

/// Runtime selection for [`PickerSecondaryAxis`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerSecondaryState {
    axis: PickerSecondaryAxis,
    selected: usize,
}

impl PickerSecondaryState {
    pub fn new(axis: PickerSecondaryAxis) -> Self {
        Self { axis, selected: 0 }
    }

    pub fn axis(&self) -> &PickerSecondaryAxis {
        &self.axis
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn selected_label(&self) -> &str {
        self.axis
            .options
            .get(self.selected)
            .map(String::as_str)
            .unwrap_or("")
    }

    pub fn select(&mut self, index: usize) {
        if self.axis.options.is_empty() {
            return;
        }
        let next = index.min(self.axis.options.len() - 1);
        if next != self.selected {
            self.selected = next;
        }
    }

    pub fn move_selection(&mut self, delta: isize) {
        if self.axis.options.is_empty() {
            return;
        }
        let len = self.axis.options.len() as isize;
        let next = (self.selected as isize + delta).rem_euclid(len) as usize;
        self.select(next);
    }
}

/// Item metadata required for fuzzy matching and optional preview text.
pub trait PickerItem {
    fn label(&self) -> Cow<'_, str>;
    fn detail(&self) -> Option<Cow<'_, str>> {
        None
    }

    /// When a secondary axis is active, return `true` if this item should appear for `option_index`.
    fn matches_secondary(&self, option_index: usize, option_label: &str) -> bool {
        let _ = (option_index, option_label);
        true
    }

    /// Short muted suffix rendered after the label (e.g. marking the active configuration).
    fn annotation(&self) -> Option<Cow<'_, str>> {
        None
    }
}

/// Result of confirming or dismissing the picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerIntent<T> {
    Selected(T),
    Cancelled,
}

/// How much terminal space the picker widget should occupy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerDisplayMode {
    /// Fill the widget's render area (default).
    Fill,
    /// Centered panel with maximum size (modal-style).
    Centered { max_width: u16, max_height: u16 },
}

impl Default for PickerDisplayMode {
    fn default() -> Self {
        Self::Fill
    }
}

/// One flattened row in the virtual list (section header or selectable item).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerRowKind {
    SectionHeader { section: usize },
    Item { section: usize, item: usize },
}

/// Flattened row used for layout and rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerRow {
    pub kind: PickerRowKind,
    /// Section title or item label (filled when rows are rebuilt).
    pub text: String,
    pub match_indices: Vec<u32>,
}

/// Layout hint written by the widget each frame (for scroll/selection keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PickerLayoutHint {
    pub list_width: u16,
    pub list_viewport_height: u16,
}

/// Stateful fuzzy picker: query, grouped items, optional secondary filter, list selection.
#[derive(Debug)]
pub struct PickerState<T: PickerItem> {
    open: bool,
    query: String,
    filter: FuzzyFilter,
    list: ListState,
    sections: Vec<PickerSection<T>>,
    secondary: Option<PickerSecondaryState>,
    rows: Vec<PickerRow>,
    layout: PickerLayoutHint,
    revision: u64,
}

impl<T: PickerItem> PickerState<T> {
    pub fn new() -> Self {
        Self {
            open: false,
            query: String::new(),
            filter: FuzzyFilter::new(),
            list: ListState::new(),
            sections: Vec::new(),
            secondary: None,
            rows: Vec::new(),
            layout: PickerLayoutHint::default(),
            revision: 0,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn sections(&self) -> &[PickerSection<T>] {
        &self.sections
    }

    pub fn rows(&self) -> &[PickerRow] {
        &self.rows
    }

    pub fn secondary(&self) -> Option<&PickerSecondaryState> {
        self.secondary.as_ref()
    }

    pub fn list_state(&self) -> &ListState {
        &self.list
    }

    pub fn list_state_mut(&mut self) -> &mut ListState {
        &mut self.list
    }

    /// Disjoint borrows for rendering the virtual list alongside row data.
    pub fn list_and_rows(&mut self) -> (&mut ListState, &[PickerRow]) {
        (&mut self.list, &self.rows)
    }

    pub fn layout_hint(&self) -> PickerLayoutHint {
        self.layout
    }

    /// Monotonic revision for layout caches (query, filter, list, rows).
    pub fn ui_revision(&self) -> u64 {
        self.revision
            .max(Revisioned::revision(&self.filter))
            .max(Component::revision(&self.list))
    }

    pub fn set_layout_hint(&mut self, hint: PickerLayoutHint) {
        self.layout = hint;
    }

    pub fn set_sections(&mut self, sections: Vec<PickerSection<T>>) {
        self.sections = sections;
        self.rebuild_rows();
        self.bump();
    }

    pub fn set_secondary_axis(&mut self, axis: Option<PickerSecondaryAxis>) {
        self.secondary = axis.map(PickerSecondaryState::new);
        self.rebuild_rows();
        self.bump();
    }

    /// Select a secondary-axis option by index (no-op when the axis is absent).
    pub fn select_secondary(&mut self, index: usize) {
        if let Some(secondary) = self.secondary.as_mut() {
            secondary.select(index);
            self.rebuild_rows();
            self.bump();
        }
    }

    pub fn open(&mut self) {
        if !self.open {
            self.open = true;
            self.query.clear();
            self.filter.set_pattern("");
            self.rebuild_rows();
            self.bump();
        }
    }

    pub fn close(&mut self) {
        if self.open {
            self.open = false;
            self.bump();
        }
    }

    pub fn open_with_trap(&mut self, focus: &mut FocusRing, trap: FocusId) {
        self.open();
        focus.push_trap(trap);
    }

    pub fn close_with_trap(&mut self, focus: &mut FocusRing) {
        self.close();
        if focus.trap_depth() > 0 {
            focus.pop_trap();
        }
    }

    /// Item referenced by the currently selected row, if any.
    pub fn selected_item(&self) -> Option<&T> {
        let row_index = self.list.selected()?;
        let row = self.rows.get(row_index)?;
        match row.kind {
            PickerRowKind::Item { section, item } => {
                self.sections.get(section).and_then(|s| s.items.get(item))
            }
            PickerRowKind::SectionHeader { .. } => None,
        }
    }

    pub fn selected_detail(&self) -> Option<Cow<'_, str>> {
        self.selected_item().and_then(|item| item.detail())
    }

    /// Item for a flattened row index, if that row is a selectable item.
    pub fn item_for_row(&self, row_index: usize) -> Option<&T> {
        let row = self.rows.get(row_index)?;
        match row.kind {
            PickerRowKind::Item { section, item } => {
                self.sections.get(section).and_then(|s| s.items.get(item))
            }
            PickerRowKind::SectionHeader { .. } => None,
        }
    }

    /// Move list selection to the first item matching `predicate`, if any.
    pub fn select_matching_item(&mut self, predicate: impl Fn(&T) -> bool) {
        let target = self.rows.iter().position(|row| {
            if let PickerRowKind::Item { section, item } = row.kind {
                self.sections
                    .get(section)
                    .and_then(|s| s.items.get(item))
                    .is_some_and(&predicate)
            } else {
                false
            }
        });
        if let Some(index) = target {
            self.list.select(Some(index));
            self.scroll_selection_into_view();
            self.bump();
        }
    }

    fn rebuild_rows(&mut self) {
        self.rows.clear();
        let secondary_index = self.secondary.as_ref().map(|s| s.selected());
        let secondary_label = self.secondary.as_ref().map(|s| s.selected_label());

        struct FlatItem {
            section: usize,
            item: usize,
            label: String,
        }

        let mut flat: Vec<FlatItem> = Vec::new();
        for (section_index, section) in self.sections.iter().enumerate() {
            for (item_index, item) in section.items.iter().enumerate() {
                if let (Some(idx), Some(label)) = (secondary_index, secondary_label) {
                    if !item.matches_secondary(idx, label) {
                        continue;
                    }
                }
                flat.push(FlatItem {
                    section: section_index,
                    item: item_index,
                    label: item.label().into_owned(),
                });
            }
        }

        let labels: Vec<&str> = flat.iter().map(|entry| entry.label.as_str()).collect();
        let ranked = self.filter.rank(&labels, labels.len());
        let mut indices_by_flat: Vec<Vec<u32>> = vec![Vec::new(); flat.len()];
        let mut matched: std::collections::HashSet<usize> = std::collections::HashSet::new();
        if self.filter.matches_everything() {
            matched.extend(0..flat.len());
        } else {
            for m in ranked {
                matched.insert(m.item_index);
                indices_by_flat[m.item_index] = m.indices;
            }
        }

        for (section_index, section) in self.sections.iter().enumerate() {
            let mut section_rows: Vec<PickerRow> = Vec::new();
            for (item_index, _) in section.items.iter().enumerate() {
                let flat_index = flat
                    .iter()
                    .position(|entry| entry.section == section_index && entry.item == item_index);
                if let Some(fi) = flat_index {
                    if matched.contains(&fi) {
                        let label = section
                            .items
                            .get(item_index)
                            .map(|item| item.label().into_owned())
                            .unwrap_or_default();
                        section_rows.push(PickerRow {
                            kind: PickerRowKind::Item {
                                section: section_index,
                                item: item_index,
                            },
                            text: label,
                            match_indices: indices_by_flat[fi].clone(),
                        });
                    }
                }
            }
            if !section_rows.is_empty() {
                self.rows.push(PickerRow {
                    kind: PickerRowKind::SectionHeader {
                        section: section_index,
                    },
                    text: section.title.clone(),
                    match_indices: Vec::new(),
                });
                self.rows.extend(section_rows);
            }
        }

        self.ensure_selection();
    }

    fn ensure_selection(&mut self) {
        let current = self.list.selected();
        if current.is_some_and(|idx| self.is_selectable_row(idx)) {
            return;
        }
        let first = self
            .rows
            .iter()
            .position(|row| self.is_selectable_row_kind(&row.kind));
        self.list.select(first);
        self.scroll_selection_into_view();
    }

    fn is_selectable_row(&self, index: usize) -> bool {
        self.rows
            .get(index)
            .is_some_and(|row| self.is_selectable_row_kind(&row.kind))
    }

    fn is_selectable_row_kind(&self, kind: &PickerRowKind) -> bool {
        matches!(kind, PickerRowKind::Item { .. })
    }

    fn move_selection(&mut self, delta: isize) {
        if self.rows.is_empty() {
            self.list.select(None);
            return;
        }
        let start = self
            .list
            .selected()
            .unwrap_or_else(|| self.first_item_row_index().unwrap_or(0));
        let len = self.rows.len();
        let mut index = start;
        for _ in 0..len {
            index = (index as isize + delta).rem_euclid(len as isize) as usize;
            if self.is_selectable_row(index) {
                self.list.select(Some(index));
                self.scroll_selection_into_view();
                self.bump();
                return;
            }
        }
    }

    fn first_item_row_index(&self) -> Option<usize> {
        self.rows
            .iter()
            .position(|row| self.is_selectable_row_kind(&row.kind))
    }

    fn scroll_selection_into_view(&mut self) {
        let Some(row) = self.list.selected() else {
            return;
        };
        let hint = self.layout;
        if hint.list_viewport_height == 0 {
            return;
        }
        let cache = self.list.height_cache();
        let start = cache.item_start(row);
        let end = start + cache.item_height(row) as usize;
        let offset = self.list.scroll().offset();
        let view_end = offset + hint.list_viewport_height as usize;
        if start < offset {
            self.list.scroll_mut().scroll_to_offset(start);
        } else if end > view_end {
            self.list
                .scroll_mut()
                .scroll_to_offset(end.saturating_sub(hint.list_viewport_height as usize));
        }
    }

    fn push_query_char(&mut self, ch: char) {
        self.query.push(ch);
        self.filter.set_pattern(&self.query);
        self.rebuild_rows();
        self.bump();
    }

    fn pop_query_char(&mut self) {
        if self.query.pop().is_some() {
            self.filter.set_pattern(&self.query);
            self.rebuild_rows();
            self.bump();
        }
    }

    fn clear_query(&mut self) {
        if !self.query.is_empty() {
            self.query.clear();
            self.filter.set_pattern("");
            self.rebuild_rows();
            self.bump();
        }
    }

    fn bump(&mut self) {
        self.revision += 1;
    }

    fn take_selected_item(&mut self) -> Option<T>
    where
        T: Clone,
    {
        let row_index = self.list.selected()?;
        let row = self.rows.get(row_index)?;
        match row.kind {
            PickerRowKind::Item { section, item } => self
                .sections
                .get(section)
                .and_then(|s| s.items.get(item))
                .cloned(),
            PickerRowKind::SectionHeader { .. } => None,
        }
    }
}

impl<T: PickerItem> Default for PickerState<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PickerItem + Clone> Component for PickerState<T> {
    type Intent = PickerIntent<T>;

    fn handle(&mut self, ev: &Event, keys: &Keymap) -> Outcome<Self::Intent> {
        if !self.open {
            return Outcome::Ignored;
        }

        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                return Outcome::Consumed;
            }

            if let Some(action) = keys.resolve(&[Scope::Modal], key) {
                if action.0 == "picker.dismiss" || action.0 == "dialog.dismiss" {
                    self.close();
                    return Outcome::Intent(PickerIntent::Cancelled);
                }
            }

            match key.code {
                KeyCode::Escape => {
                    self.close();
                    Outcome::Intent(PickerIntent::Cancelled)
                }
                KeyCode::Enter => {
                    if let Some(item) = self.take_selected_item() {
                        self.close();
                        Outcome::Intent(PickerIntent::Selected(item))
                    } else {
                        Outcome::Consumed
                    }
                }
                KeyCode::Up => {
                    self.move_selection(-1);
                    Outcome::Consumed
                }
                KeyCode::Down => {
                    self.move_selection(1);
                    Outcome::Consumed
                }
                KeyCode::Left if self.secondary.is_some() => {
                    self.secondary.as_mut().unwrap().move_selection(-1);
                    self.rebuild_rows();
                    self.bump();
                    Outcome::Consumed
                }
                KeyCode::Right if self.secondary.is_some() => {
                    self.secondary.as_mut().unwrap().move_selection(1);
                    self.rebuild_rows();
                    self.bump();
                    Outcome::Consumed
                }
                KeyCode::Tab if self.secondary.is_some() => {
                    let delta = if key.modifiers.shift { -1 } else { 1 };
                    self.secondary.as_mut().unwrap().move_selection(delta);
                    self.rebuild_rows();
                    self.bump();
                    Outcome::Consumed
                }
                KeyCode::Backspace => {
                    self.pop_query_char();
                    Outcome::Consumed
                }
                KeyCode::Char('u')
                    if key.modifiers
                        == KeyModifiers {
                            control: true,
                            ..KeyModifiers::NONE
                        } =>
                {
                    self.clear_query();
                    Outcome::Consumed
                }
                KeyCode::Char(c) if !key.modifiers.control && !key.modifiers.alt => {
                    self.push_query_char(c);
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
            .max(self.filter.revision())
            .max(Component::revision(&self.list))
    }
}
