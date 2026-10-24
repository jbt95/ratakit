use ratakit_core::{
    Component, DialogIntent, DialogState, Event, FocusId, FocusRing, KeyCode, KeyEvent,
    KeyEventKind, Keymap, Outcome, TableState, TabsState,
};

#[test]
fn dialog_confirm_and_dismiss() {
    let mut state = DialogState::new();
    state.set_button_count(2);
    state.open();
    let keys = Keymap::empty();

    let enter = Event::Key(KeyEvent::new(KeyCode::Enter));
    assert_eq!(
        state.handle(&enter, &keys),
        Outcome::Intent(DialogIntent::Confirmed(0))
    );
    assert!(!state.is_open());

    state.open();
    let esc = Event::Key(KeyEvent::new(KeyCode::Escape));
    assert_eq!(
        state.handle(&esc, &keys),
        Outcome::Intent(DialogIntent::Dismissed)
    );
}

#[test]
fn dialog_focus_trap() {
    let mut focus = FocusRing::new([FocusId(1), FocusId(2)]);
    let mut dialog = DialogState::new();
    dialog.set_button_count(1);
    dialog.open_with_trap(&mut focus, FocusId(99));
    assert_eq!(focus.focused(), Some(FocusId(99)));
    assert_eq!(focus.trap_depth(), 1);

    dialog.close_with_trap(&mut focus);
    assert_eq!(focus.trap_depth(), 0);
}

#[test]
fn tabs_cycle_with_keyboard() {
    let mut tabs = TabsState::new();
    tabs.set_tab_count(3);
    let keys = Keymap::empty();

    let right = Event::Key(KeyEvent::new(KeyCode::Right));
    assert_eq!(tabs.handle(&right, &keys), Outcome::Intent(1));
    assert_eq!(tabs.selected(), 1);

    let left = Event::Key(KeyEvent::new(KeyCode::Left));
    assert_eq!(tabs.handle(&left, &keys), Outcome::Intent(0));
}

#[test]
fn table_sort_toggle_and_selection() {
    let mut table = TableState::new();
    table.set_dimensions(10, 3);
    table.toggle_sort(1);
    assert_eq!(table.sort_column(), Some(1));
    assert_eq!(
        table.sort_direction(),
        ratakit_core::SortDirection::Ascending
    );
    table.toggle_sort(1);
    assert_eq!(
        table.sort_direction(),
        ratakit_core::SortDirection::Descending
    );

    let keys = Keymap::empty();
    let down = Event::Key(KeyEvent::new(KeyCode::Down));
    assert_eq!(table.handle(&down, &keys), Outcome::Consumed);
    assert_eq!(table.selected_row(), Some(0));

    table.select_row(Some(9));
    table.ensure_visible(5);
    assert_eq!(table.offset(), 5);
}

#[test]
fn dialog_digit_shortcut() {
    let mut state = DialogState::new();
    state.set_button_count(3);
    state.open();
    let keys = Keymap::empty();
    let two = Event::Key(KeyEvent {
        code: KeyCode::Char('2'),
        modifiers: ratakit_core::KeyModifiers::NONE,
        kind: KeyEventKind::Press,
    });
    assert_eq!(
        state.handle(&two, &keys),
        Outcome::Intent(DialogIntent::Confirmed(1))
    );
}
