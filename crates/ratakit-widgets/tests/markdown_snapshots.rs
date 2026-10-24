use ratakit_core::Theme;
use ratakit_testing::assert_stateful_widget_snapshot;
use ratakit_text::{MarkdownOptions, SyntectHighlighter};
use ratakit_widgets::{Markdown, MarkdownState};

fn sample_markdown() -> String {
    [
        "# Hello ratakit",
        "",
        "A **bold** note with [docs](https://example.com).",
        "",
        "> Quoted wisdom",
        "",
        "- first item",
        "- second item",
        "",
        "| Col A | Col B |",
        "| ----- | ----- |",
        "| one   | two   |",
        "",
        "```rust",
        "fn main() {",
        "    println!(\"hi\");",
        "}",
        "```",
        "",
        "Tail line.",
    ]
    .join("\n")
}

#[test]
fn markdown_sample_80x18_dark() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let highlighter = SyntectHighlighter;
    let mut state = MarkdownState::new(MarkdownOptions::default());
    state.push_delta(&sample_markdown());
    state.finish();
    let widget = Markdown::new(&theme, &highlighter);
    assert_stateful_widget_snapshot!("markdown_sample_80x18_dark", widget, &mut state, 80, 18);
}
