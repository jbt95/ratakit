use ratakit_core::Theme;
use ratakit_testing::assert_widget_snapshot;
use ratakit_text::{LangId, SyntectHighlighter};
use ratakit_widgets::{CodeBlock, CodeBlockState};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::block::Block;

fn rust_sample() -> CodeBlockState {
    let mut state = CodeBlockState::new();
    state.set_language(LangId("Rust".to_string()));
    state.set_source("fn main() {\n    println!(\"hi\");\n}\n");
    state
}

struct CodeBlockShot<'a> {
    theme: &'a Theme,
    highlighter: &'a SyntectHighlighter,
    state: &'a mut CodeBlockState,
}

impl Widget for CodeBlockShot<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let widget = CodeBlock::new(self.theme, self.highlighter);
        widget.render(area, buf, self.state);
    }
}

#[test]
fn code_block_rust_80x6_dark() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let highlighter = SyntectHighlighter;
    let mut state = rust_sample();
    assert_widget_snapshot!(
        "code_block_rust_80x6_dark",
        CodeBlockShot {
            theme: &theme,
            highlighter: &highlighter,
            state: &mut state,
        },
        80,
        6
    );
}

#[test]
fn code_block_rust_40x8_dark() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let highlighter = SyntectHighlighter;
    let mut state = rust_sample();
    assert_widget_snapshot!(
        "code_block_rust_40x8_dark",
        CodeBlockShot {
            theme: &theme,
            highlighter: &highlighter,
            state: &mut state,
        },
        40,
        8
    );
}

struct Bordered<W>(W);

impl<W: Widget> Widget for Bordered<W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner = Block::bordered().title("main.rs").inner(area);
        self.0.render(inner, buf);
    }
}

#[test]
fn code_block_with_border_80x7() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let highlighter = SyntectHighlighter;
    let mut state = rust_sample();
    let inner = CodeBlockShot {
        theme: &theme,
        highlighter: &highlighter,
        state: &mut state,
    };
    assert_widget_snapshot!("code_block_bordered_80x7", Bordered(inner), 80, 7);
}
