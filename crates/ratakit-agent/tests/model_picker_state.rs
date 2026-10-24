#![cfg(feature = "widgets")]

use ratakit_agent::model::{Effort, ModelCatalogEntry, ModelChoice};
use ratakit_agent::widgets::{ModelPicker, ModelPickerIntent, ModelPickerState};
use ratakit_core::{
    Component, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, Keymap, Outcome, Theme,
};
use ratakit_testing::render_stateful;

fn sample_catalog() -> Vec<ModelCatalogEntry> {
    vec![
        ModelCatalogEntry::new("OpenAI", "gpt-4.1")
            .with_detail("General reasoning")
            .with_efforts(vec![Effort::Medium, Effort::High]),
        ModelCatalogEntry::new("OpenAI", "gpt-4.1-mini").with_efforts(vec![Effort::Low]),
        ModelCatalogEntry::new("Anthropic", "claude-sonnet-4")
            .with_detail("Balanced")
            .with_efforts(vec![Effort::Low, Effort::Medium, Effort::High]),
    ]
}

fn press(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
    })
}

#[test]
fn effort_axis_filters_models() {
    let mut state = ModelPickerState::new();
    state.set_catalog(sample_catalog());
    state.open();
    assert_eq!(state.selected_effort_label(), Some("low"));
    assert_eq!(
        state.selected_model_label().as_deref(),
        Some("gpt-4.1-mini")
    );
    let _ = state.handle(&press(KeyCode::Right), &Keymap::empty());
    assert_eq!(state.selected_effort_label(), Some("medium"));
    assert_eq!(state.selected_model_label().as_deref(), Some("gpt-4.1"));
}

#[test]
fn fuzzy_query_narrows_models() {
    let mut state = ModelPickerState::new();
    state.set_catalog(sample_catalog());
    state.open();
    for ch in "claude".chars() {
        let _ = state.handle(&press(KeyCode::Char(ch)), &Keymap::empty());
    }
    let rendered = render_stateful(ModelPicker::new(&Theme::dark()), 56, 14, &mut state);
    assert!(rendered.contains("claude-sonnet-4"));
    assert!(!rendered.contains("gpt-4.1-mini"));
}

#[test]
fn enter_returns_model_choice_with_effort() {
    let mut state = ModelPickerState::new();
    state.set_catalog(sample_catalog());
    state.set_current(Some(ModelChoice {
        provider: "OpenAI".into(),
        model: "gpt-4.1".into(),
        effort: Some(Effort::Medium),
    }));
    state.open();
    let outcome = state.handle(&press(KeyCode::Enter), &Keymap::empty());
    assert_eq!(
        outcome,
        Outcome::Intent(ModelPickerIntent::Selected(ModelChoice {
            provider: "OpenAI".into(),
            model: "gpt-4.1".into(),
            effort: Some(Effort::Medium),
        }))
    );
    assert!(!state.is_open());
}

#[test]
fn marks_current_configuration() {
    let mut state = ModelPickerState::new();
    state.set_catalog(sample_catalog());
    state.set_current(Some(ModelChoice {
        provider: "OpenAI".into(),
        model: "gpt-4.1".into(),
        effort: Some(Effort::Medium),
    }));
    state.open();
    let rendered = render_stateful(ModelPicker::new(&Theme::dark()), 56, 14, &mut state);
    assert!(rendered.contains(" · current"));
}
