use ratakit_core::Theme;
use ratakit_testing::assert_stateful_widget_snapshot;
use ratakit_text::{LangId, SyntectHighlighter};
use ratakit_widgets::{CodeBlock, CodeBlockState};
use ratatui_widgets::block::Block;

fn rust_sample() -> CodeBlockState {
    let mut state = CodeBlockState::new();
    state.set_language(LangId("Rust".to_string()));
    state.set_source("fn main() {\n    println!(\"hi\");\n}\n");
    state
}

#[test]
fn code_block_rust_80x6_dark() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let highlighter = SyntectHighlighter;
    let mut state = rust_sample();
    let widget = CodeBlock::new(&theme, &highlighter);
    assert_stateful_widget_snapshot!("code_block_rust_80x6_dark", widget, &mut state, 80, 6);
}

#[test]
fn code_block_rust_40x8_dark() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let highlighter = SyntectHighlighter;
    let mut state = rust_sample();
    let widget = CodeBlock::new(&theme, &highlighter);
    assert_stateful_widget_snapshot!("code_block_rust_40x8_dark", widget, &mut state, 40, 8);
}

#[test]
fn code_block_with_border_80x7() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let highlighter = SyntectHighlighter;
    let mut state = rust_sample();
    let widget = CodeBlock::new(&theme, &highlighter).block(Block::bordered().title("main.rs"));
    assert_stateful_widget_snapshot!("code_block_bordered_80x7", widget, &mut state, 80, 7);
}
