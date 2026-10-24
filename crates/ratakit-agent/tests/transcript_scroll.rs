use ratakit_agent::state::{AgentEvent, MessageId, TranscriptState};
use ratakit_agent::widgets::Transcript;
use ratakit_core::{
    Component, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, Keymap, Theme,
};
use ratakit_testing::{assert_stateful_widget_snapshot, render_stateful_widget};
use ratakit_text::SyntectHighlighter;

fn mid(s: &str) -> MessageId {
    MessageId::new(s)
}

#[test]
fn follow_tail_pins_end_until_user_scrolls_up() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let mut state = TranscriptState::new();
    state.apply(AgentEvent::AssistantDelta {
        id: mid("a"),
        text: "line one\nline two\nline three\nline four\nline five".into(),
    });
    render_stateful_widget(
        Transcript::new(&theme, &SyntectHighlighter),
        &mut state,
        40,
        3,
    );
    assert!(state.list().scroll().follow_tail());
    let offset_before = state.list().scroll().offset();

    state.apply(AgentEvent::AssistantDelta {
        id: mid("a"),
        text: "\nline six\nline seven".into(),
    });
    render_stateful_widget(
        Transcript::new(&theme, &SyntectHighlighter),
        &mut state,
        40,
        3,
    );
    assert!(state.list().scroll().follow_tail());
    assert!(
        state.list().scroll().offset() >= offset_before,
        "tail should stay pinned as content grows"
    );

    let up = Event::Key(KeyEvent {
        code: KeyCode::Up,
        kind: KeyEventKind::Press,
        modifiers: KeyModifiers::default(),
    });
    let _ = state.handle(&up, &Keymap::empty());
    assert!(!state.list().scroll().follow_tail());
}

#[test]
fn follow_tail_snapshot_scrolled_up_40x4() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let widget = Transcript::new(&theme, &SyntectHighlighter).scrollbar(true);
    let mut state = TranscriptState::new();
    for i in 0..6 {
        state.apply(AgentEvent::Notice {
            level: ratakit_agent::state::Level::Info,
            text: format!("notice row {i} with padding"),
        });
    }
    render_stateful_widget(
        Transcript::new(&theme, &SyntectHighlighter).scrollbar(true),
        &mut state,
        40,
        4,
    );
    let content_height = state.list().layout().content_height;
    state
        .list_mut()
        .scroll_mut()
        .scroll_by(-4, 4, content_height);
    assert_stateful_widget_snapshot!(
        "transcript_follow_tail_released_40x4",
        widget,
        &mut state,
        40,
        4
    );
}
