use proptest::prelude::*;
use ratakit_core::Theme;
use ratakit_text::{MarkdownOptions, MarkdownStream, PlainHighlighter};

fn render_whole(source: &str, width: u16) -> String {
    let theme = Theme::dark();
    let hl = PlainHighlighter::default();
    let mut stream = MarkdownStream::new(MarkdownOptions::default());
    stream.push(source);
    stream.finish();
    stream.lines_signature(width, &theme, &hl)
}

fn char_boundary_at(source: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    source
        .char_indices()
        .nth(char_index)
        .map_or(source.len(), |(idx, _)| idx)
}

fn render_streamed(source: &str, width: u16, char_splits: &[usize]) -> String {
    let theme = Theme::dark();
    let hl = PlainHighlighter::default();
    let mut stream = MarkdownStream::new(MarkdownOptions::default());
    let char_len = source.chars().count();
    let mut boundaries: Vec<usize> = char_splits
        .iter()
        .map(|idx| char_boundary_at(source, (*idx).min(char_len)))
        .collect();
    boundaries.push(0);
    boundaries.push(source.len());
    boundaries.sort_unstable();
    boundaries.dedup();
    for window in boundaries.windows(2) {
        let chunk = &source[window[0]..window[1]];
        if !chunk.is_empty() {
            stream.push(chunk);
        }
    }
    stream.finish();
    stream.lines_signature(width, &theme, &hl)
}

proptest! {
    #[test]
    fn streaming_splits_match_whole_render(
        source in "\\PC*",
        splits in prop::collection::vec(0usize..40, 0..12),
    ) {
        let width = 48u16;
        let whole = render_whole(&source, width);
        let streamed = render_streamed(&source, width, &splits);
        prop_assert_eq!(whole, streamed);
    }
}

#[test]
fn fence_and_blank_line_commits() {
    let theme = Theme::dark();
    let hl = PlainHighlighter::default();
    let mut stream = MarkdownStream::new(MarkdownOptions::default());
    stream.push("# Title\n\n");
    stream.push("```rust\nlet x = 1;\n```\n\n");
    stream.push("Tail paragraph");
    stream.finish();
    let sig = stream.lines_signature(60, &theme, &hl);
    assert!(sig.contains("Title"));
    assert!(sig.contains("Tail"));
}
