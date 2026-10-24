#![cfg(feature = "fuzzy")]

use ratakit_core::Keymap;
use ratakit_core::{
    Component, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, Outcome, PickerIntent,
    PickerItem, PickerSecondaryAxis, PickerSection, PickerState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemoItem {
    label: String,
    detail: Option<String>,
    effort: String,
}

impl PickerItem for DemoItem {
    fn label(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(&self.label)
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        self.detail.as_deref().map(std::borrow::Cow::Borrowed)
    }

    fn matches_secondary(&self, _option_index: usize, option_label: &str) -> bool {
        self.effort == option_label
    }
}

fn sample_sections() -> Vec<PickerSection<DemoItem>> {
    vec![
        PickerSection {
            title: "OpenAI".into(),
            items: vec![
                DemoItem {
                    label: "gpt-4.1".into(),
                    detail: Some("General reasoning model".into()),
                    effort: "medium".into(),
                },
                DemoItem {
                    label: "gpt-4.1-mini".into(),
                    detail: None,
                    effort: "low".into(),
                },
            ],
        },
        PickerSection {
            title: "Anthropic".into(),
            items: vec![DemoItem {
                label: "claude-sonnet".into(),
                detail: Some("Balanced".into()),
                effort: "medium".into(),
            }],
        },
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
fn fuzzy_query_filters_items() {
    let mut state = PickerState::new();
    state.set_sections(sample_sections());
    state.open();
    for ch in "claude".chars() {
        let _ = state.handle(&press(KeyCode::Char(ch)), &Keymap::empty());
    }
    assert_eq!(state.rows().len(), 2); // header + one item
    assert!(state.selected_item().is_some());
    assert_eq!(
        state.selected_item().map(|i| i.label.as_str()),
        Some("claude-sonnet")
    );
}

#[test]
fn secondary_axis_filters_effort() {
    let mut state = PickerState::new();
    state.set_sections(sample_sections());
    state.set_secondary_axis(Some(PickerSecondaryAxis {
        label: "Effort".into(),
        options: vec!["low".into(), "medium".into()],
    }));
    state.open();
    assert_eq!(state.rows().len(), 2); // low effort: header + gpt-4.1-mini
    assert_eq!(
        state.selected_item().map(|i| i.label.as_str()),
        Some("gpt-4.1-mini")
    );
    let _ = state.handle(&press(KeyCode::Right), &Keymap::empty());
    assert_eq!(state.secondary().unwrap().selected_label(), "medium");
    assert_eq!(state.rows().len(), 4);
}

#[test]
fn enter_emits_selected_item() {
    let mut state = PickerState::new();
    state.set_sections(sample_sections());
    state.open();
    let outcome = state.handle(&press(KeyCode::Enter), &Keymap::empty());
    assert_eq!(
        outcome,
        Outcome::Intent(PickerIntent::Selected(DemoItem {
            label: "gpt-4.1".into(),
            detail: Some("General reasoning model".into()),
            effort: "medium".into(),
        }))
    );
    assert!(!state.is_open());
}

#[test]
fn escape_cancels() {
    let mut state = PickerState::new();
    state.set_sections(sample_sections());
    state.open();
    let outcome = state.handle(&press(KeyCode::Escape), &Keymap::empty());
    assert_eq!(outcome, Outcome::Intent(PickerIntent::Cancelled));
    assert!(!state.is_open());
}
