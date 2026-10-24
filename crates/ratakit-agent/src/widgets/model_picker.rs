//! Model picker backed by [`FuzzyPicker`](ratakit_widgets::FuzzyPicker).

use std::borrow::Cow;
use std::collections::BTreeSet;

use ratakit_core::{
    Component, Event, FocusId, FocusRing, Keymap, Outcome, PickerDisplayMode, PickerIntent,
    PickerItem, PickerSecondaryAxis, PickerSection, PickerState, Theme,
};
use ratakit_widgets::FuzzyPicker;
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;

use crate::model::{Effort, ModelCatalogEntry, ModelChoice};

/// Result of confirming or dismissing the model picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelPickerIntent {
    Selected(ModelChoice),
    Cancelled,
}

#[derive(Clone, Debug)]
struct ModelPickerRow {
    provider: String,
    model: String,
    detail: Option<String>,
    efforts: Vec<Effort>,
    is_current: bool,
}

impl PickerItem for ModelPickerRow {
    fn label(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.model)
    }

    fn detail(&self) -> Option<Cow<'_, str>> {
        self.detail.as_deref().map(Cow::Borrowed)
    }

    fn matches_secondary(&self, _option_index: usize, option_label: &str) -> bool {
        if self.efforts.is_empty() {
            return true;
        }
        self.efforts
            .iter()
            .any(|effort| effort.as_str() == option_label)
    }

    fn annotation(&self) -> Option<Cow<'_, str>> {
        if self.is_current {
            Some(Cow::Borrowed("current"))
        } else {
            None
        }
    }
}

/// Stateful model picker: providers as sections, fuzzy filter, optional effort axis.
#[derive(Debug)]
pub struct ModelPickerState {
    picker: PickerState<ModelPickerRow>,
    catalog: Vec<ModelCatalogEntry>,
    current: Option<ModelChoice>,
}

impl ModelPickerState {
    pub fn new() -> Self {
        Self {
            picker: PickerState::new(),
            catalog: Vec::new(),
            current: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.picker.is_open()
    }

    pub fn current(&self) -> Option<&ModelChoice> {
        self.current.as_ref()
    }

    pub fn set_current(&mut self, choice: Option<ModelChoice>) {
        self.current = choice;
        self.rebuild_from_catalog();
    }

    pub fn set_catalog(&mut self, entries: Vec<ModelCatalogEntry>) {
        self.catalog = entries;
        self.rebuild_from_catalog();
    }

    pub fn open(&mut self) {
        self.picker.open();
        self.align_to_current();
    }

    pub fn close(&mut self) {
        self.picker.close();
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

    fn rebuild_from_catalog(&mut self) {
        self.picker
            .set_secondary_axis(secondary_axis_for_catalog(&self.catalog));
        self.refresh_current_markers();
    }

    fn align_to_current(&mut self) {
        let Some(choice) = self.current.clone() else {
            return;
        };
        if let Some(effort) = &choice.effort {
            if let Some(axis) = self.picker.secondary() {
                let index = axis
                    .axis()
                    .options
                    .iter()
                    .position(|label| label == effort.as_str());
                if let Some(index) = index {
                    self.picker.select_secondary(index);
                }
            }
        }
        let provider = choice.provider.clone();
        let model = choice.model.clone();
        let effort = choice.effort.clone();
        self.picker.select_matching_item(|row| {
            row.provider == provider
                && row.model == model
                && row_matches_choice_effort(row, effort.as_ref())
        });
        self.refresh_current_markers();
    }

    /// Label of the currently highlighted model row, if any.
    pub fn selected_model_label(&self) -> Option<String> {
        self.picker
            .selected_item()
            .map(|row| row.label().into_owned())
    }

    /// Selected effort chip label when the catalog defines effort levels.
    pub fn selected_effort_label(&self) -> Option<&str> {
        self.picker.secondary().map(|axis| axis.selected_label())
    }

    fn refresh_current_markers(&mut self) {
        let secondary_label = self.picker.secondary().map(|axis| axis.selected_label());
        let sections = build_sections(&self.catalog, &self.current, secondary_label);
        self.picker.set_sections(sections);
    }

    fn choice_from_selection(&self) -> Option<ModelChoice> {
        let row = self.picker.selected_item()?;
        let effort = if row.efforts.is_empty() {
            None
        } else {
            self.picker
                .secondary()
                .and_then(|axis| Effort::parse(axis.selected_label()))
        };
        Some(ModelChoice {
            provider: row.provider.clone(),
            model: row.model.clone(),
            effort,
        })
    }
}

impl Default for ModelPickerState {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ModelPickerState {
    type Intent = ModelPickerIntent;

    fn handle(&mut self, ev: &Event, keys: &Keymap) -> Outcome<Self::Intent> {
        let secondary_before = self.picker.secondary().map(|axis| axis.selected());
        let outcome = self.picker.handle(ev, keys);
        let secondary_after = self.picker.secondary().map(|axis| axis.selected());
        if secondary_before != secondary_after {
            self.refresh_current_markers();
        }
        match outcome {
            Outcome::Intent(PickerIntent::Selected(_)) => {
                if let Some(choice) = self.choice_from_selection() {
                    Outcome::Intent(ModelPickerIntent::Selected(choice))
                } else {
                    Outcome::Consumed
                }
            }
            Outcome::Intent(PickerIntent::Cancelled) => {
                Outcome::Intent(ModelPickerIntent::Cancelled)
            }
            Outcome::Consumed => Outcome::Consumed,
            Outcome::Ignored => Outcome::Ignored,
        }
    }

    fn revision(&self) -> u64 {
        self.picker.revision()
    }
}

fn row_matches_choice_effort(row: &ModelPickerRow, effort: Option<&Effort>) -> bool {
    if row.efforts.is_empty() {
        return effort.is_none();
    }
    match effort {
        Some(e) => row.efforts.iter().any(|allowed| allowed == e),
        None => false,
    }
}

fn is_current_row(
    current: &ModelChoice,
    row: &ModelPickerRow,
    secondary_label: Option<&str>,
) -> bool {
    if current.provider != row.provider || current.model != row.model {
        return false;
    }
    if row.efforts.is_empty() {
        return current.effort.is_none();
    }
    match (&current.effort, secondary_label) {
        (Some(effort), Some(label)) => effort.as_str() == label,
        (None, _) | (Some(_), None) => false,
    }
}

fn build_sections(
    catalog: &[ModelCatalogEntry],
    current: &Option<ModelChoice>,
    secondary_label: Option<&str>,
) -> Vec<PickerSection<ModelPickerRow>> {
    let mut sections: Vec<PickerSection<ModelPickerRow>> = Vec::new();
    for entry in catalog {
        let placeholder = row_placeholder(entry);
        let row = ModelPickerRow {
            provider: entry.provider.clone(),
            model: entry.model.clone(),
            detail: entry.detail.clone(),
            efforts: entry.efforts.clone(),
            is_current: current
                .as_ref()
                .is_some_and(|choice| is_current_row(choice, &placeholder, secondary_label)),
        };
        if let Some(section) = sections.iter_mut().find(|s| s.title == entry.provider) {
            section.items.push(row);
        } else {
            sections.push(PickerSection {
                title: entry.provider.clone(),
                items: vec![row],
            });
        }
    }
    sections
}

fn row_placeholder(entry: &ModelCatalogEntry) -> ModelPickerRow {
    ModelPickerRow {
        provider: entry.provider.clone(),
        model: entry.model.clone(),
        detail: entry.detail.clone(),
        efforts: entry.efforts.clone(),
        is_current: false,
    }
}

fn secondary_axis_for_catalog(catalog: &[ModelCatalogEntry]) -> Option<PickerSecondaryAxis> {
    let mut efforts: BTreeSet<Effort> = BTreeSet::new();
    for entry in catalog {
        for effort in &entry.efforts {
            efforts.insert(effort.clone());
        }
    }
    if efforts.is_empty() {
        return None;
    }
    let mut sorted: Vec<Effort> = efforts.into_iter().collect();
    sorted.sort_by_key(Effort::sort_key);
    let options = sorted
        .into_iter()
        .map(|effort| effort.as_str().to_string())
        .collect();
    Some(PickerSecondaryAxis {
        label: "Effort".into(),
        options,
    })
}

/// Full-viewport fuzzy model picker (uses [`PickerDisplayMode::Fill`] — not a separate mode name).
pub struct ModelPicker<'a> {
    title: &'a str,
    theme: &'a Theme,
    preview: bool,
}

impl<'a> ModelPicker<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            title: "Choose model",
            theme,
            preview: true,
        }
    }

    #[must_use]
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    #[must_use]
    pub fn preview(mut self, show: bool) -> Self {
        self.preview = show;
        self
    }
}

impl StatefulWidget for ModelPicker<'_> {
    type State = ModelPickerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner = FuzzyPicker::new(self.title, self.theme)
            .preview(self.preview)
            .display_mode(PickerDisplayMode::Fill);
        inner.render(area, buf, &mut state.picker);
    }
}
