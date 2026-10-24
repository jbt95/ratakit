use ratakit_core::{Component, Event, KeyCode, KeyEvent, Keymap, Outcome, TextBuffer};
use ratakit_widgets::{TextAreaState, TextIntent};

#[test]
fn enter_submits_plain_text() {
    let mut state = TextAreaState::with_buffer(TextBuffer::new());
    state.buffer.insert_str("hi");
    let keys = Keymap::empty();
    let ev = Event::Key(KeyEvent::new(KeyCode::Enter));
    let outcome = state.handle(&ev, &keys);
    assert_eq!(
        outcome,
        Outcome::Intent(TextIntent::Submit {
            text: "hi".to_owned(),
        })
    );
}
