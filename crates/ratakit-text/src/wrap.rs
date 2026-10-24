//! Display-width wrapping for plain text and [`StyledLine`] (with caches).

use std::collections::HashMap;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::styled::{StyledLine, StyledSpan, TextStyle};

/// One visual line in a soft-wrapped plain-text view.
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

        let Some(ch) = text[i..].chars().next() else {
            break;
        };
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
    (last, lines.last().map_or(0, |l| l.text.len()))
}

/// Wrap a single logical line to fit `width` columns (0 = no wrap, returns clone).
pub fn wrap_styled_line(line: &StyledLine, width: u16) -> Vec<StyledLine> {
    if width == 0 {
        return vec![line.clone()];
    }
    let max = usize::from(width);
    let mut out = Vec::new();
    let mut current = StyledLine::new();
    let mut col = 0usize;

    for span in &line.spans {
        for grapheme in span.text.graphemes(true) {
            let g_width = grapheme_width(grapheme);
            if col > 0 && col + g_width > max {
                out.push(current);
                current = StyledLine::new();
                col = 0;
            }
            if grapheme == "\n" {
                out.push(current);
                current = StyledLine::new();
                col = 0;
                continue;
            }
            push_grapheme(&mut current, grapheme, span.style);
            col += g_width;
        }
    }
    if !current.spans.is_empty() || out.is_empty() {
        out.push(current);
    }
    out
}

fn grapheme_width(g: &str) -> usize {
    g.chars()
        .next()
        .map_or(0, |c| UnicodeWidthChar::width(c).unwrap_or(0))
}

fn push_grapheme(line: &mut StyledLine, grapheme: &str, style: TextStyle) {
    if let Some(last) = line.spans.last_mut()
        && last.style == style
    {
        last.text.push_str(grapheme);
        return;
    }
    line.push(StyledSpan {
        text: grapheme.to_string(),
        style,
    });
}

/// Cache wrapped lines keyed by `(source revision, width)`.
#[derive(Debug, Default)]
pub struct WrapCache {
    revision: u64,
    width: u16,
    lines: Vec<StyledLine>,
}

impl WrapCache {
    /// Return cached wrapped lines, recomputing when `revision` or `width` change.
    pub fn lines(&mut self, revision: u64, width: u16, source: &[StyledLine]) -> &[StyledLine] {
        if self.revision == revision && self.width == width && !self.lines.is_empty() {
            return &self.lines;
        }
        self.revision = revision;
        self.width = width;
        self.lines.clear();
        for line in source {
            self.lines.extend(wrap_styled_line(line, width));
        }
        &self.lines
    }

    /// Invalidate without changing revision (e.g. after manual clear).
    pub fn clear(&mut self) {
        self.lines.clear();
    }
}

/// Wrap many logical lines and flatten to visual rows.
pub fn wrap_styled_lines(lines: &[StyledLine], width: u16) -> Vec<StyledLine> {
    let mut out = Vec::new();
    for line in lines {
        out.extend(wrap_styled_line(line, width));
    }
    out
}

/// Per-source-line wrap cache (for code blocks with stable line indices).
#[derive(Debug, Default)]
pub struct PerLineWrapCache {
    width: u16,
    revision: u64,
    rows: HashMap<usize, Vec<StyledLine>>,
}

impl PerLineWrapCache {
    /// Wrapped visual rows for one source line index.
    pub fn wrap_line(
        &mut self,
        revision: u64,
        width: u16,
        line_index: usize,
        line: &StyledLine,
    ) -> &[StyledLine] {
        if self.revision != revision || self.width != width {
            self.rows.clear();
            self.revision = revision;
            self.width = width;
        }
        self.rows
            .entry(line_index)
            .or_insert_with(|| wrap_styled_line(line, width));
        self.rows.get(&line_index).map_or(&[], |v| v.as_slice())
    }

    pub fn clear(&mut self) {
        self.rows.clear();
    }
}
