#![cfg(feature = "fuzzy")]

use ratakit_core::{CompletionInsert, CompletionItem, CompletionState, Theme};
use ratakit_widgets::{CompletionMenu, CompletionMenuState, TextArea, TextAreaState};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

fn sample_catalog() -> Vec<CompletionItem> {
    vec![
        CompletionItem::new("/clear", CompletionInsert::Text("/clear ".into()))
            .with_detail("Reset conversation"),
        CompletionItem::new("/model", CompletionInsert::Text("/model ".into()))
            .with_detail("Pick a model"),
        CompletionItem::new(
            "@src/main.rs",
            CompletionInsert::Mention {
                label: "@src/main.rs".into(),
            },
        ),
    ]
}

fn render_composer(
    width: u16,
    height: u16,
    buffer_text: &str,
    completion: &mut CompletionState,
) -> String {
    let theme = Theme::dark();
    let mut textarea = TextAreaState::new();
    textarea.buffer.insert_str(buffer_text);
    completion.sync_from_buffer(&textarea.buffer);

    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, width, height);
            let textarea_widget = TextArea::new(&theme).block_title("composer");
            textarea_widget.render(area, frame.buffer_mut(), &mut textarea);
            let menu_widget = CompletionMenu::new(&theme, &mut textarea, completion).max_rows(4);
            let mut menu_state = CompletionMenuState::default();
            menu_widget.render(area, frame.buffer_mut(), &mut menu_state);
        })
        .expect("draw");
    let buffer = terminal.backend().buffer();
    ratakit_testing::buffer_to_string(buffer)
}

#[test]
fn completion_menu_below_cursor_40x8() {
    let mut completion = CompletionState::new();
    completion.set_catalog(sample_catalog());
    completion.set_max_visible_rows(4);
    let rendered = render_composer(40, 8, "@sr", &mut completion);
    insta::assert_snapshot!("completion_menu_below_40x8", rendered);
}

#[test]
fn completion_menu_above_cursor_40x8() {
    let mut completion = CompletionState::new();
    completion.set_catalog(sample_catalog());
    completion.set_max_visible_rows(4);
    let text = "first line\nsecond line\nthird\n@mo";
    let rendered = render_composer(40, 8, text, &mut completion);
    insta::assert_snapshot!("completion_menu_above_40x8", rendered);
}

#[test]
fn completion_menu_slash_filter_40x6() {
    let mut completion = CompletionState::new();
    completion.set_catalog(sample_catalog());
    completion.set_max_visible_rows(3);
    let rendered = render_composer(40, 6, "/mo", &mut completion);
    insta::assert_snapshot!("completion_menu_slash_40x6", rendered);
}
