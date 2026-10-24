// Commit-boundary logic adapted from openai/codex `codex-rs/tui/src/markdown_stream.rs`
// (Apache-2.0). Modifications Copyright 2026 The ratakit Authors.

use ratakit_core::{Revisioned, Theme};

use crate::highlight::Highlighter;
use crate::markdown_render::{
    MarkdownOptions, MarkdownStyles, StreamingCodeState, lines_signature, render_markdown,
};
use crate::styled::StyledLine;
use crate::wrap::WrapCache;

/// Incremental markdown accumulator with block-stable commits.
#[derive(Debug)]
pub struct MarkdownStream {
    buffer: String,
    committed_end: usize,
    blocks: Vec<CommittedBlock>,
    options: MarkdownOptions,
    revision: u64,
    finished: bool,
    open_logical: Vec<StyledLine>,
    open_wrap: WrapCache,
    open_code: StreamingCodeState,
    cached_width: u16,
    cached_theme_revision: u64,
    cached_output: Vec<StyledLine>,
    cached_revision: u64,
}

#[derive(Debug)]
struct CommittedBlock {
    source: String,
    logical: Vec<StyledLine>,
    wrap: WrapCache,
    block_revision: u64,
}

impl MarkdownStream {
    /// Create an empty stream.
    pub fn new(options: MarkdownOptions) -> Self {
        Self {
            buffer: String::new(),
            committed_end: 0,
            blocks: Vec::new(),
            options,
            revision: 0,
            finished: false,
            open_logical: Vec::new(),
            open_wrap: WrapCache::default(),
            open_code: StreamingCodeState::default(),
            cached_width: 0,
            cached_theme_revision: 0,
            cached_output: Vec::new(),
            cached_revision: 0,
        }
    }

    /// Append a streaming delta (`O(tail)` re-parse on the open tail only).
    pub fn push(&mut self, delta: &str) {
        if delta.is_empty() {
            return;
        }
        self.buffer.push_str(delta);
        self.revision += 1;
        self.finished = false;
        self.try_commit_stable_blocks();
        self.open_logical.clear();
        self.open_wrap.clear();
    }

    /// Mark the stream complete and commit any remaining source as a final block.
    pub fn finish(&mut self) {
        if self.committed_end < self.buffer.len() {
            let tail = self.buffer[self.committed_end..].to_string();
            self.commit_block(tail);
            self.committed_end = self.buffer.len();
        }
        self.finished = true;
        self.revision += 1;
        self.open_logical.clear();
        self.open_wrap.clear();
        self.open_code.clear();
    }

    pub fn source(&self) -> &str {
        &self.buffer
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Logical lines for the open (uncommitted) tail, mainly for tests.
    pub fn open_tail_logical(
        &mut self,
        theme: &Theme,
        highlighter: &dyn Highlighter,
    ) -> &[StyledLine] {
        self.ensure_open_logical(theme, highlighter);
        &self.open_logical
    }

    /// Wrapped visual lines for the current width and theme.
    pub fn lines(
        &mut self,
        width: u16,
        theme: &Theme,
        highlighter: &dyn Highlighter,
    ) -> &[StyledLine] {
        let theme_revision = theme.theme_revision();
        if self.cached_revision == self.revision
            && self.cached_width == width
            && self.cached_theme_revision == theme_revision
            && !self.cached_output.is_empty()
        {
            return &self.cached_output;
        }

        self.ensure_open_logical(theme, highlighter);
        let styles = MarkdownStyles::from_theme(theme);
        let mut out = Vec::new();
        for block in &mut self.blocks {
            let logical = if block.block_revision == theme_revision {
                &block.logical
            } else {
                block.logical = render_markdown(
                    &block.source,
                    &self.options,
                    &styles,
                    highlighter,
                    &mut StreamingCodeState::default(),
                );
                block.block_revision = theme_revision;
                &block.logical
            };
            let wrapped = block.wrap.lines(self.revision, width, logical);
            out.extend_from_slice(wrapped);
        }
        let wrapped_open = self
            .open_wrap
            .lines(self.revision, width, &self.open_logical);
        out.extend_from_slice(wrapped_open);

        self.cached_output = out;
        self.cached_width = width;
        self.cached_theme_revision = theme_revision;
        self.cached_revision = self.revision;
        &self.cached_output
    }

    /// Test helper: signature of wrapped output at `width`.
    pub fn lines_signature(
        &mut self,
        width: u16,
        theme: &Theme,
        highlighter: &dyn Highlighter,
    ) -> String {
        lines_signature(self.lines(width, theme, highlighter))
    }

    fn ensure_open_logical(&mut self, theme: &Theme, highlighter: &dyn Highlighter) {
        if !self.open_logical.is_empty() || self.committed_end >= self.buffer.len() {
            if self.committed_end >= self.buffer.len() {
                self.open_logical.clear();
                self.open_code.clear();
            }
            return;
        }
        let tail = &self.buffer[self.committed_end..];
        let styles = MarkdownStyles::from_theme(theme);
        self.open_logical = render_markdown(
            tail,
            &self.options,
            &styles,
            highlighter,
            &mut self.open_code,
        );
    }

    fn try_commit_stable_blocks(&mut self) {
        while let Some(end) = stable_commit_end(&self.buffer, self.committed_end) {
            let slice = self.buffer[self.committed_end..end].to_string();
            self.commit_block(slice);
            self.committed_end = end;
            self.open_code.clear();
        }
    }

    fn commit_block(&mut self, source: String) {
        if source.is_empty() {
            return;
        }
        self.blocks.push(CommittedBlock {
            source,
            logical: Vec::new(),
            wrap: WrapCache::default(),
            block_revision: u64::MAX,
        });
    }
}

impl Revisioned for MarkdownStream {
    fn revision(&self) -> u64 {
        self.revision
    }
}

/// Returns the byte index **after** the newly committed region, or `None` if no stable boundary yet.
fn stable_commit_end(buffer: &str, from: usize) -> Option<usize> {
    if from >= buffer.len() {
        return None;
    }
    let slice = &buffer[from..];
    let mut in_fence = false;
    let mut pos = 0usize;
    while pos < slice.len() {
        let line_end = slice[pos..].find('\n').map_or(slice.len(), |i| pos + i + 1);
        let line = &slice[pos..line_end];
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            if !in_fence {
                return Some(from + line_end);
            }
        } else if !in_fence && trimmed.trim().is_empty() && pos > 0 {
            return Some(from + line_end);
        }
        if line_end == slice.len() {
            break;
        }
        pos = line_end;
    }
    None
}
