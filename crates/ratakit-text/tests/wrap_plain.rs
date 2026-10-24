use ratakit_text::{cursor_wrapped_position, wrap_lines};

#[test]
fn plain_wrap_matches_display_width() {
    let lines = wrap_lines("hello world", 5);
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].text, "hello");
    assert_eq!(lines[1].text, " worl");
    assert_eq!(lines[2].text, "d");
}

#[test]
fn cursor_position_in_wrapped_plain() {
    let lines = wrap_lines("ab cd", 3);
    let (row, col) = cursor_wrapped_position(&lines, 4);
    assert_eq!(row, 1);
    assert_eq!(col, 1);
}
