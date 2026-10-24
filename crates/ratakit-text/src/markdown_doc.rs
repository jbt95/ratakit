use ratakit_core::{Revisioned, Theme};

use crate::highlight::Highlighter;
use crate::markdown_render::MarkdownOptions;
use crate::markdown_stream::MarkdownStream;
use crate::styled::StyledLine;

/// Static markdown document (whole source committed).
#[derive(Debug)]
pub struct MarkdownDoc {
    stream: MarkdownStream,
}

impl MarkdownDoc {
    /// Parse and commit `source` immediately.
    pub fn new(source: impl Into<String>, options: MarkdownOptions) -> Self {
        let text = source.into();
        let mut stream = MarkdownStream::new(options);
        stream.push(&text);
        stream.finish();
        Self { stream }
    }

    pub fn source(&self) -> &str {
        self.stream.source()
    }

    pub fn lines(
        &mut self,
        width: u16,
        theme: &Theme,
        highlighter: &dyn Highlighter,
    ) -> &[StyledLine] {
        self.stream.lines(width, theme, highlighter)
    }

    pub fn as_stream(&self) -> &MarkdownStream {
        &self.stream
    }

    pub fn as_stream_mut(&mut self) -> &mut MarkdownStream {
        &mut self.stream
    }

    /// Take ownership of the inner stream.
    pub fn into_stream(self) -> MarkdownStream {
        self.stream
    }
}

impl Revisioned for MarkdownDoc {
    fn revision(&self) -> u64 {
        self.stream.revision()
    }
}
