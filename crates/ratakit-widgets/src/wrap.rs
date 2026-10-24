use unicode_width::UnicodeWidthStr;

/// One visual line in a soft-wrapped view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedLine {
    /// Start offset in the source display string.
    pub start: usize,
    pub text: String,
}

/// Soft-wrap `text` to at most `width` columns (Unicode display width).
pub fn wrap_lines(text: &str, width: u16) -> Vec<WrappedLine> {
    let width = width.max(1) as usize;
    if text.is_empty() {
        return vec![WrappedLine {
            start: 0,
            text: String::new(),
        }];
    }

    let mut lines = Vec::new();
    let mut line_start = 0;
    let mut line = String::new();
    let mut line_width = 0;
    let mut i = 0;

    while i < text.len() {
        if text[i..].starts_with('\n') {
            lines.push(WrappedLine {
                start: line_start,
                text: line,
            });
            line = String::new();
            line_width = 0;
            i += 1;
            line_start = i;
            continue;
        }

        let ch = text[i..].chars().next().unwrap();
        let ch_len = ch.len_utf8();
        let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());

        if line_width + ch_width > width && !line.is_empty() {
            lines.push(WrappedLine {
                start: line_start,
                text: line,
            });
            line = String::new();
            line_width = 0;
            line_start = i;
        }

        line.push(ch);
        line_width += ch_width;
        i += ch_len;
    }

    lines.push(WrappedLine {
        start: line_start,
        text: line,
    });
    lines
}

/// Map a display-string cursor offset to `(line, column)` in wrapped layout.
pub fn cursor_wrapped_position(lines: &[WrappedLine], cursor: usize) -> (usize, usize) {
    for (idx, line) in lines.iter().enumerate() {
        let end = line.start + line.text.len();
        if cursor <= end {
            let col = cursor.saturating_sub(line.start);
            return (idx, col);
        }
    }
    let last = lines.len().saturating_sub(1);
    (last, lines.last().map(|l| l.text.len()).unwrap_or(0))
}
