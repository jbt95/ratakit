//! Syntax-highlighted code block widget.

use std::ops::Range;

use ratakit_core::Revisioned;
use ratakit_core::Theme;
use ratakit_text::{Highlighter, HlState, LangId, PerLineWrapCache, StyledLine, TextStyle};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::block::Block;
use ratatui_widgets::paragraph::Paragraph;

use crate::styled_convert::{styled_line_to_ratatui_owned, text_style_to_ratatui};

/// Stateful data for [`CodeBlock`]: source text, language, scroll, and highlight range.
#[derive(Debug, Default)]
pub struct CodeBlockState {
    source: String,
    language: Option<LangId>,
    revision: u64,
    scroll_row: usize,
    highlight_lines: Option<Range<usize>>,
    line_states: Vec<HlState>,
    wrap_cache: PerLineWrapCache,
    last_width: u16,
    last_theme_revision: u64,
}

impl CodeBlockState {
    /// Empty block.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace source and bump [`revision`](Revisioned::revision).
    pub fn set_source(&mut self, source: impl Into<String>) {
        self.source = source.into();
        self.line_states.clear();
        self.wrap_cache.clear();
        self.revision += 1;
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Set language explicitly (e.g. from markdown fence info).
    pub fn set_language(&mut self, lang: LangId) {
        self.language = Some(lang);
        self.line_states.clear();
        self.revision += 1;
    }

    /// Detect language from path extension via `highlighter`.
    pub fn detect_language_from_path(
        &mut self,
        path: &std::path::Path,
        highlighter: &dyn Highlighter,
    ) {
        if let Some(lang) = highlighter.language_for_path(path) {
            self.set_language(lang);
        }
    }

    /// Highlighted source line range (inclusive start, exclusive end).
    pub fn set_highlight_lines(&mut self, range: Option<Range<usize>>) {
        self.highlight_lines = range;
        self.revision += 1;
    }

    pub fn highlight_lines(&self) -> Option<Range<usize>> {
        self.highlight_lines.clone()
    }

    /// Scroll offset in visual rows.
    pub fn set_scroll_row(&mut self, row: usize) {
        self.scroll_row = row;
    }

    pub fn scroll_row(&self) -> usize {
        self.scroll_row
    }

    pub fn language(&self) -> Option<&LangId> {
        self.language.as_ref()
    }

    fn source_lines(&self) -> Vec<&str> {
        if self.source.is_empty() {
            return vec![""];
        }
        self.source.split('\n').collect()
    }

    fn ensure_line_states(&mut self, highlighter: &dyn Highlighter, line_count: usize) {
        if self.line_states.len() == line_count {
            return;
        }
        let lang = self
            .language
            .clone()
            .unwrap_or_else(|| LangId("Plain Text".to_string()));
        self.line_states = (0..line_count)
            .map(|_| highlighter.new_state(lang.clone()))
            .collect();
    }
}

impl Revisioned for CodeBlockState {
    fn revision(&self) -> u64 {
        self.revision
    }
}

/// Renders syntax-highlighted source with optional line numbers and range highlight.
pub struct CodeBlock<'a> {
    theme: &'a Theme,
    highlighter: &'a dyn Highlighter,
    show_line_numbers: bool,
    block: Option<Block<'a>>,
}

impl<'a> CodeBlock<'a> {
    /// Create a code block with the given theme and highlighter.
    pub fn new(theme: &'a Theme, highlighter: &'a dyn Highlighter) -> Self {
        Self {
            theme,
            highlighter,
            show_line_numbers: true,
            block: None,
        }
    }

    /// Show gutter line numbers (default: true).
    pub fn line_numbers(mut self, on: bool) -> Self {
        self.show_line_numbers = on;
        self
    }

    /// Optional surrounding [`Block`].
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl StatefulWidget for CodeBlock<'_> {
    type State = CodeBlockState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner = self.block.as_ref().map_or(area, |b| b.inner(area));
        if inner.is_empty() {
            if let Some(block) = &self.block {
                block.render(area, buf);
            }
            return;
        }

        let gutter = if self.show_line_numbers {
            gutter_width(state.source_lines().len())
        } else {
            0
        };
        let code_width = inner.width.saturating_sub(gutter);
        state.last_width = code_width;
        state.last_theme_revision = self.theme.theme_revision();

        let lines: Vec<String> = state
            .source_lines()
            .into_iter()
            .map(str::to_string)
            .collect();
        state.ensure_line_states(self.highlighter, lines.len());

        let muted = text_style_to_ratatui(TextStyle::fg(self.theme.palette.muted));
        let highlight_bg = Style::new()
            .bg(Color::Rgb(0x31, 0x32, 0x44))
            .add_modifier(Modifier::DIM);

        let mut visual_rows: Vec<Line<'static>> = Vec::new();
        for (line_idx, line_text) in lines.iter().map(String::as_str).enumerate() {
            let hl_state = &mut state.line_states[line_idx];
            let spans = self.highlighter.highlight_line(hl_state, line_text);
            let styled = if spans.is_empty() {
                StyledLine::raw(line_text)
            } else {
                let mut sl = StyledLine::new();
                for s in spans {
                    sl.push(s);
                }
                sl
            };

            let in_highlight = state
                .highlight_lines
                .as_ref()
                .is_some_and(|r| line_idx >= r.start && line_idx < r.end);

            let wrapped: Vec<StyledLine> = state
                .wrap_cache
                .wrap_line(state.revision, code_width, line_idx, &styled)
                .to_vec();
            for (wrap_idx, wline) in wrapped.iter().enumerate() {
                let mut rat_line = if gutter > 0 && wrap_idx == 0 {
                    let num = format!("{:>width$} ", line_idx + 1, width = gutter as usize - 1);
                    let mut combined = Line::from(Span::styled(num, muted));
                    let code_part = styled_line_to_ratatui_owned(wline);
                    for span in code_part.iter() {
                        combined.push_span(span.clone());
                    }
                    combined
                } else if gutter > 0 {
                    let pad = " ".repeat(gutter as usize);
                    let mut combined = Line::from(Span::styled(pad, muted));
                    let code_part = styled_line_to_ratatui_owned(wline);
                    for span in code_part.iter() {
                        combined.push_span(span.clone());
                    }
                    combined
                } else {
                    styled_line_to_ratatui_owned(wline)
                };
                if in_highlight {
                    rat_line = rat_line.style(highlight_bg);
                }
                visual_rows.push(rat_line);
            }
        }

        let scroll = state.scroll_row.min(visual_rows.len().saturating_sub(1));
        state.scroll_row = scroll;

        let scroll_y = u16::try_from(scroll).unwrap_or(u16::MAX);
        let paragraph = Paragraph::new(visual_rows).scroll((scroll_y, 0));
        paragraph.render(inner, buf);

        if let Some(block) = &self.block {
            block.render(area, buf);
        }
    }
}

fn gutter_width(line_count: usize) -> u16 {
    let digits = line_count
        .checked_ilog10()
        .map_or(0, |d| d as usize)
        + 1;
    let digits = if line_count == 0 { 1 } else { digits };
    u16::try_from(digits + 1).unwrap_or(4)
}
