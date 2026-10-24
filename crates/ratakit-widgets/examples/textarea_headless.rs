//! Headless demo: render a [`TextArea`] via TestBackend (CI-friendly).

use ratakit_core::Theme;
use ratakit_testing::render_stateful;
use ratakit_widgets::{TextArea, TextAreaState};

fn main() {
    let theme = Theme::dark();
    let mut state = TextAreaState::new();
    state.buffer.insert_str("ratakit textarea");

    let widget = TextArea::new(&theme)
        .placeholder("Type a message…")
        .block_title("composer");

    let rendered = render_stateful(widget, 40, 6, &mut state);
    assert!(rendered.contains("ratakit"));
    println!("ok");
}
