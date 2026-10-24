use ratakit_text::{
    StyledLine, StyledSpan, TextModifier, TextStyle, wrap_styled_line, wrap_styled_lines,
};

#[test]
fn wrap_splits_long_plain_line() {
    let line = StyledLine::raw("hello world");
    let wrapped = wrap_styled_line(&line, 5);
    assert_eq!(wrapped.len(), 3);
    assert_eq!(wrapped[0].plain_text(), "hello");
    assert_eq!(wrapped[1].plain_text(), " worl");
    assert_eq!(wrapped[2].plain_text(), "d");
}

#[test]
fn wrap_preserves_style_boundaries() {
    let mut line = StyledLine::new();
    line.push(StyledSpan::styled("ab", TextStyle::PLAIN));
    line.push(StyledSpan::styled(
        "cd",
        TextStyle::PLAIN.with_modifier(TextModifier::BOLD),
    ));
    let wrapped = wrap_styled_line(&line, 3);
    assert_eq!(wrapped.len(), 2);
    assert_eq!(wrapped[0].plain_text(), "abc");
    assert_eq!(wrapped[1].plain_text(), "d");
}

#[test]
fn wrap_many_lines_flattens() {
    let lines = vec![StyledLine::raw("aa"), StyledLine::raw("bb")];
    let out = wrap_styled_lines(&lines, 1);
    assert_eq!(out.len(), 4);
}
