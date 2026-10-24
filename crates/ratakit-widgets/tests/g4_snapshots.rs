use std::time::{Duration, Instant};

use ratakit_core::{
    ActionLabels, Keymap, KeymapPreset, Motion, MotionMode, Theme, Toast, ToastLevel, ToastQueue,
    key_hints_from_keymap,
};
use ratakit_testing::{assert_stateful_widget_snapshot, assert_widget_snapshot};
use ratakit_widgets::{KeyHints, Spinner, StatusBar, StatusSegment, Toasts};

fn dark_theme() -> Theme {
    Theme::dark()
}

fn light_theme() -> Theme {
    Theme::light()
}

#[test]
fn spinner_full_unicode_80x1() {
    let theme = dark_theme();
    let mut motion = Motion::new();
    motion.set_mode(MotionMode::Full);
    let widget = Spinner::new(&theme).label("Loading");
    assert_stateful_widget_snapshot!("spinner_full_dark_80x1", widget, &mut motion, 80, 1);
}

#[test]
fn spinner_reduced_80x1() {
    let mut theme = dark_theme();
    theme.with_motion(MotionMode::Reduced);
    let mut motion = Motion::new();
    let widget = Spinner::new(&theme).label("Loading");
    assert_stateful_widget_snapshot!("spinner_reduced_dark_80x1", widget, &mut motion, 80, 1);
}

#[test]
fn spinner_ascii_40x1() {
    let mut theme = light_theme();
    theme.with_glyphs(ratakit_core::GlyphSet::Ascii);
    let mut motion = Motion::new();
    let widget = Spinner::new(&theme).label("Wait");
    assert_stateful_widget_snapshot!("spinner_ascii_light_40x1", widget, &mut motion, 40, 1);
}

#[test]
fn status_bar_truncation_80x1() {
    let theme = dark_theme();
    let segments = [
        StatusSegment::left("main", 10),
        StatusSegment::center("context", 5),
        StatusSegment::right("branch main", 8),
        StatusSegment::right("100% context", 3),
    ];
    let widget = StatusBar::new(&theme, &segments);
    assert_widget_snapshot!("status_bar_dark_80x1", widget, 80, 1);
}

#[test]
fn status_bar_wide_120x1() {
    let theme = light_theme();
    let segments = [
        StatusSegment::left("agent", 10),
        StatusSegment::center("idle", 5),
        StatusSegment::right("esc cancel", 9),
    ];
    let widget = StatusBar::new(&theme, &segments);
    assert_widget_snapshot!("status_bar_light_120x1", widget, 120, 1);
}

#[test]
fn key_hints_80x1() {
    let theme = dark_theme();
    let map = Keymap::from_preset(KeymapPreset::Default);
    let mut labels = ActionLabels::new();
    labels.insert("focus.next", "next");
    labels.insert("focus.prev", "prev");
    let hints = key_hints_from_keymap(&map, &labels, 4);
    let widget = KeyHints::new(&theme, &hints);
    assert_widget_snapshot!("key_hints_dark_80x1", widget, 80, 1);
}

#[test]
fn toasts_stack_80x3() {
    let theme = dark_theme();
    let start = Instant::now();
    let mut queue = ToastQueue::new();
    queue.push(Toast::persistent("Saved", ToastLevel::Success));
    queue.push(Toast::new(
        "Syncing…",
        ToastLevel::Info,
        start,
        Duration::from_secs(60),
    ));
    queue.push(Toast::persistent("Network slow", ToastLevel::Warning));
    let widget = Toasts::new(&theme).max_visible(3);
    assert_stateful_widget_snapshot!("toasts_dark_80x3", widget, &mut queue, 80, 3);
}
