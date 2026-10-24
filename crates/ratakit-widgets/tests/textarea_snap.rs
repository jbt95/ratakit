use ratakit_core::Theme;
use ratakit_testing::render_stateful;
use ratakit_widgets::{TextArea, TextAreaState};

#[test]
fn textarea_empty_with_placeholder_40x6() {
    let theme = Theme::dark();
    let mut state = TextAreaState::new();
    let widget = TextArea::new(&theme)
        .placeholder("Type a message…")
        .block_title("composer");
    let rendered = render_stateful(widget, 40, 6, &mut state);
    insta::assert_snapshot!("textarea_empty_40x6", rendered);
}

#[test]
fn textarea_with_text_40x6() {
    let theme = Theme::dark();
    let mut state = TextAreaState::new();
    state.buffer.insert_str("hello\nworld");
    let widget = TextArea::new(&theme).block_title("composer");
    let rendered = render_stateful(widget, 40, 6, &mut state);
    insta::assert_snapshot!("textarea_with_text_40x6", rendered);
}

#[test]
fn textarea_with_paste_atom_40x6() {
    let theme = Theme::dark();
    let mut state = TextAreaState::new();
    let paste = "line\n".repeat(10);
    state.buffer.insert_paste(&paste);
    let widget = TextArea::new(&theme).block_title("composer");
    let rendered = render_stateful(widget, 40, 6, &mut state);
    insta::assert_snapshot!("textarea_paste_atom_40x6", rendered);
}

#[test]
fn textarea_screen_reader_uses_a_text_label_without_box_borders() {
    let mut theme = Theme::dark();
    theme.enable_screen_reader();
    let mut state = TextAreaState::new();
    let widget = TextArea::new(&theme)
        .placeholder("Message input")
        .block_title("Composer")
        .bordered(false);
    let rendered = render_stateful(widget, 40, 4, &mut state);

    assert!(rendered.contains("Composer:"));
    assert!(rendered.contains("Message input"));
    assert!(!rendered.contains('╭'));
    assert!(!rendered.contains('╰'));
}
