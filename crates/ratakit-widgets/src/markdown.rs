//! Markdown widget backed by [`MarkdownStream`] / [`MarkdownDoc`].

use ratakit_core::{Revisioned, Theme};
use ratakit_text::{Highlighter, MarkdownDoc, MarkdownOptions, MarkdownStream, StyledLine};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::Line;
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::block::Block;
use ratatui_widgets::paragraph::Paragraph;

use crate::styled_convert::{styled_line_to_ratatui_owned, text_style_to_ratatui};

/// Stateful markdown view (streaming or static).
#[derive(Debug)]
pub struct MarkdownState {
    stream: MarkdownStream,
    scroll_row: usize,
}

impl MarkdownState {
    /// Empty streaming document.
    pub fn new(options: MarkdownOptions) -> Self {
        Self {
            stream: MarkdownStream::new(options),
            scroll_row: 0,
        }
    }

    /// Wrap an existing stream.
    pub fn from_stream(stream: MarkdownStream) -> Self {
        Self {
            stream,
            scroll_row: 0,
        }
    }

    /// Static document committed in one shot.
    pub fn from_doc(doc: MarkdownDoc) -> Self {
        Self::from_stream(doc.into_stream())
    }

    pub fn push_delta(&mut self, delta: &str) {
        self.stream.push(delta);
    }

    pub fn finish(&mut self) {
        self.stream.finish();
    }

    pub fn source(&self) -> &str {
        self.stream.source()
    }

    pub fn stream(&self) -> &MarkdownStream {
        &self.stream
    }

    pub fn stream_mut(&mut self) -> &mut MarkdownStream {
        &mut self.stream
    }

    pub fn set_scroll_row(&mut self, row: usize) {
        self.scroll_row = row;
    }

    pub fn scroll_row(&self) -> usize {
        self.scroll_row
    }
}

impl Revisioned for MarkdownState {
    fn revision(&self) -> u64 {
        self.stream.revision()
    }
}

/// Renders markdown with optional border.
pub struct Markdown<'a> {
    theme: &'a Theme,
    highlighter: &'a dyn Highlighter,
    block: Option<Block<'a>>,
}

impl<'a> Markdown<'a> {
    /// Create with theme and highlighter.
    pub fn new(theme: &'a Theme, highlighter: &'a dyn Highlighter) -> Self {
        Self {
            theme,
            highlighter,
            block: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl StatefulWidget for Markdown<'_> {
    type State = MarkdownState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner = self.block.as_ref().map_or(area, |b| b.inner(area));
        if inner.is_empty() {
            if let Some(block) = &self.block {
                block.render(area, buf);
            }
            return;
        }

        let width = inner.width;
        let lines = state.stream.lines(width, self.theme, self.highlighter);
        let rat_lines: Vec<Line<'static>> = lines
            .iter()
            .map(|line| paint_line_with_links(line))
            .collect();

        let scroll = state.scroll_row.min(rat_lines.len().saturating_sub(1));
        state.scroll_row = scroll;
        let scroll_y = u16::try_from(scroll).unwrap_or(u16::MAX);

        Paragraph::new(rat_lines)
            .scroll((scroll_y, 0))
            .render(inner, buf);

        if let Some(block) = &self.block {
            block.render(area, buf);
        }
    }
}

fn paint_line_with_links(line: &StyledLine) -> Line<'static> {
    let mut out = Line::default();
    for span in &line.spans {
        if let Some(url) = &span.link {
            let osc = format!(
                "\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\",
                escape_osc8_url(url),
                span.text
            );
            out.push_span(ratatui_core::text::Span::styled(
                osc,
                text_style_to_ratatui(span.style),
            ));
        } else {
            let part = styled_line_to_ratatui_owned(&StyledLine {
                spans: vec![span.clone()],
            });
            for s in part.spans {
                out.push_span(s);
            }
        }
    }
    out
}

fn escape_osc8_url(url: &str) -> String {
    url.replace('\\', "\\\\").replace(';', "%3B")
}
