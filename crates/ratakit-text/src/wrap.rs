//! Grapheme-aware wrapping for [`StyledLine`] with width-keyed cache.

use std::collections::HashMap;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;

use crate::styled::{StyledLine, StyledSpan, TextStyle};

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
