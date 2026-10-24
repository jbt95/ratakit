#![cfg(feature = "fuzzy")]

use ratakit_core::{
    CompletionInsert, CompletionItem, CompletionState, CompletionTrigger, CompletionTriggers,
    Component, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, Keymap, Outcome, TextBuffer,
};

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
    })
}

#[test]
fn fuzzy_filter_narrows_catalog() {
    let mut buffer = TextBuffer::new();
    buffer.insert_str("@fo");
    let items = vec![
        CompletionItem::new("foo.rs", CompletionInsert::Text("foo.rs".into())),
        CompletionItem::new("bar.rs", CompletionInsert::Text("bar.rs".into())),
    ];
    let mut state = CompletionState::new();
    state.set_catalog(items);
    state.sync_from_buffer(&buffer);
    assert_eq!(state.ranked_matches().len(), 1);
    assert_eq!(state.selected_item().unwrap().label, "foo.rs");
}

#[test]
fn keyboard_navigation_wraps() {
    let mut buffer = TextBuffer::new();
    buffer.insert_str("@");
    let items = vec![
        CompletionItem::new("a", CompletionInsert::Text("a".into())),
        CompletionItem::new("b", CompletionInsert::Text("b".into())),
    ];
    let mut state = CompletionState::new();
    state.set_catalog(items);
    state.sync_from_buffer(&buffer);
    assert_eq!(state.selected_index(), 0);
    assert_eq!(
        state.handle(&key(KeyCode::Down), &Keymap::empty()),
        Outcome::Consumed
    );
    assert_eq!(state.selected_index(), 1);
    assert_eq!(
        state.handle(&key(KeyCode::Down), &Keymap::empty()),
        Outcome::Consumed
    );
    assert_eq!(state.selected_index(), 0);
}

#[test]
fn custom_triggers() {
    let mut buffer = TextBuffer::new();
    buffer.insert_str("#tag");
    let triggers = CompletionTriggers {
        triggers: vec![CompletionTrigger::new('#')],
    };
    let mut state = CompletionState::with_triggers(triggers);
    state.sync_from_buffer(&buffer);
    assert!(state.is_open());
    assert_eq!(state.session().unwrap().trigger, '#');
}
