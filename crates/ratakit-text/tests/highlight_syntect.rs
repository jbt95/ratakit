#![cfg(feature = "highlight-syntect")]

use ratakit_text::{Highlighter, SyntectHighlighter};

#[test]
fn rust_keyword_gets_color() {
    let hl = SyntectHighlighter;
    let lang = hl.language_for_name("rust").expect("rust grammar");
    let mut state = hl.new_state(lang);
    let spans = hl.highlight_line(&mut state, "fn main() {}");
    assert!(!spans.is_empty());
    assert!(spans.iter().any(|s| s.style.fg.is_some()));
}

#[test]
fn path_extension_resolves() {
    let hl = SyntectHighlighter;
    let lang = hl
        .language_for_path(std::path::Path::new("foo.rs"))
        .expect("rs extension");
    assert_eq!(lang.0.to_ascii_lowercase(), "rust");
}
