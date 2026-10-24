use std::time::{Duration, Instant};

use ratakit_core::{
    ActionLabels, Component, Event, KeyChord, KeyCode, Keymap, KeymapPreset, Motion, MotionMode,
    Revisioned, Scope, Toast, ToastLevel, ToastQueue, format_chord, key_hints_from_keymap,
};

#[test]
fn motion_reduced_uses_static_label() {
    let mut theme = ratakit_core::Theme::dark();
    theme.with_motion(MotionMode::Reduced);
    let motion = Motion::new();
    assert_eq!(motion.indicator(&theme), "working…");
}

#[test]
fn motion_tick_advances_in_full_mode() {
    let mut motion = Motion::new();
    motion.set_mode(MotionMode::Full);
    let before = Revisioned::revision(&motion);
    let outcome = motion.handle(&Event::Tick(Instant::now()), &Keymap::empty());
    assert!(outcome.is_consumed());
    assert!(Revisioned::revision(&motion) > before);
}

#[test]
fn toast_queue_expires_on_tick() {
    let start = Instant::now();
    let mut queue = ToastQueue::new();
    queue.push(Toast::new(
        "gone",
        ToastLevel::Info,
        start,
        Duration::from_millis(1),
    ));
    let later = start + Duration::from_secs(1);
    queue.handle(&Event::Tick(later), &Keymap::empty());
    assert!(queue.is_empty());
}

#[test]
fn key_hints_from_default_preset() {
    let map = Keymap::from_preset(KeymapPreset::Default);
    let mut labels = ActionLabels::new();
    labels.insert("focus.next", "next field");
    let hints = key_hints_from_keymap(&map, &labels, 8);
    assert!(!hints.is_empty());
    assert!(
        hints
            .iter()
            .any(|h| h.chord == format_chord(&KeyChord::new(KeyCode::Tab)))
    );
}

#[test]
fn bindings_list_includes_global_tab() {
    let map = Keymap::from_preset(KeymapPreset::Default);
    let bindings = map.bindings_in_scope(Scope::Global);
    assert!(!bindings.is_empty());
}
