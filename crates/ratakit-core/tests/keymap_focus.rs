use ratakit_core::{
    Event, FocusId, FocusRing, KeyChord, KeyCode, KeyEvent, KeyModifiers, Keymap, KeymapPreset,
    Scope,
};

#[test]
fn keymap_resolves_modal_before_global() {
    let mut map = Keymap::from_preset(KeymapPreset::Default);
    map.bind(
        Scope::Modal,
        KeyChord::new(KeyCode::Enter),
        "modal.confirm".into(),
    );
    map.bind(
        Scope::Global,
        KeyChord::new(KeyCode::Enter),
        "global.enter".into(),
    );

    let key = KeyEvent::new(KeyCode::Enter);
    let action = map.resolve_default(&key).expect("binding");
    assert_eq!(action.0, "modal.confirm");
}

#[test]
fn focus_ring_cycles_and_traps() {
    let mut ring = FocusRing::new([FocusId(1), FocusId(2), FocusId(3)]);
    assert_eq!(ring.focused(), Some(FocusId(1)));
    ring.next();
    assert_eq!(ring.focused(), Some(FocusId(2)));
    ring.push_trap(FocusId(99));
    assert_eq!(ring.focused(), Some(FocusId(99)));
    ring.pop_trap();
    assert_eq!(ring.focused(), Some(FocusId(2)));
}

#[test]
fn tab_with_shift_resolves_focus_prev() {
    let map = Keymap::from_preset(KeymapPreset::Default);
    let key = KeyEvent::new(KeyCode::Tab).with_modifiers(KeyModifiers {
        shift: true,
        ..KeyModifiers::NONE
    });
    let action = map.resolve_default(&key).expect("binding");
    assert_eq!(action.0, "focus.prev");
}

#[test]
fn resize_event_equality() {
    let a = Event::Resize {
        width: 80,
        height: 24,
    };
    let b = Event::Resize {
        width: 80,
        height: 24,
    };
    assert_eq!(a, b);
}
