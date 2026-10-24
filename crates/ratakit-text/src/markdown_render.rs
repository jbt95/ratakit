// Rendering logic adapted from openai/codex `codex-rs/tui/src/markdown_render.rs`
// and `markdown.rs` (Apache-2.0). Modifications Copyright 2026 The ratakit Authors.

use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratakit_core::Theme;
use unicode_width::UnicodeWidthStr;

use crate::highlight::{Highlighter, HlState};
use crate::styled::{StyledLine, StyledSpan, TextModifier, TextStyle};

/// Markdown rendering preferences.
#[derive(Debug, Clone, Default)]
pub struct MarkdownOptions {
    /// Language hint when a fenced block has no info string.
    pub default_code_language: Option<String>,
}

/// Semantic styles derived from a [`Theme`] for markdown output.
#[derive(Debug, Clone)]
pub struct MarkdownStyles {
    pub body: TextStyle,
    pub h1: TextStyle,
    pub h2: TextStyle,
    pub h3: TextStyle,
    pub h4: TextStyle,
    pub h5: TextStyle,
    pub h6: TextStyle,
    pub code_inline: TextStyle,
    pub emphasis: TextStyle,
    pub strong: TextStyle,
    pub strikethrough: TextStyle,
    pub link: TextStyle,
    pub blockquote: TextStyle,
    pub list_marker: TextStyle,
    pub table_border: TextStyle,
}

impl MarkdownStyles {
    /// Build palette-relative styles from `theme`.
    pub fn from_theme(theme: &Theme) -> Self {
        let p = &theme.palette;
        let heading = |level: u8| {
            let base = TextStyle::fg(p.foreground).with_modifier(TextModifier::BOLD);
            match level {
                1 => base.with_modifier(TextModifier::BOLD | TextModifier::UNDERLINE),
                2 => base,
                3 => base.with_modifier(TextModifier::BOLD | TextModifier::ITALIC),
                _ => base.with_modifier(TextModifier::ITALIC),
            }
        };
        Self {
            body: TextStyle::fg(p.foreground),
            h1: heading(1),
            h2: heading(2),
            h3: heading(3),
            h4: heading(4),
            h5: heading(5),
            h6: heading(6),
            code_inline: TextStyle::fg(p.accent).with_bg(p.background),
            emphasis: TextStyle::fg(p.foreground).with_modifier(TextModifier::ITALIC),
            strong: TextStyle::fg(p.foreground).with_modifier(TextModifier::BOLD),
            strikethrough: TextStyle::fg(p.muted),
            link: TextStyle::fg(p.accent).with_modifier(TextModifier::UNDERLINE),
            blockquote: TextStyle::fg(p.muted).with_modifier(TextModifier::ITALIC),
            list_marker: TextStyle::fg(p.accent),
            table_border: TextStyle::fg(p.border),
        }
    }
}

/// Incremental highlight state for one open fenced code block in a stream tail.
#[derive(Debug, Default)]
pub struct StreamingCodeState {
    lang: Option<String>,
    line_states: Vec<HlState>,
    lines: Vec<String>,
}

impl StreamingCodeState {
    /// Reset when the fence closes or the tail is re-parsed from scratch.
    pub fn clear(&mut self) {
        self.lang = None;
        self.line_states.clear();
        self.lines.clear();
    }
}

/// Parse `source` into logical (pre-wrap) styled lines.
pub fn render_markdown(
    source: &str,
    options: &MarkdownOptions,
    styles: &MarkdownStyles,
    highlighter: &dyn Highlighter,
    streaming_code: &mut StreamingCodeState,
) -> Vec<StyledLine> {
    let mut parse_options = Options::empty();
    parse_options.insert(Options::ENABLE_STRIKETHROUGH);
    parse_options.insert(Options::ENABLE_TABLES);
    parse_options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(source, parse_options);
    let mut writer = Writer {
        styles,
        highlighter,
        streaming_code,
        lines: Vec::new(),
        current: StyledLine::new(),
        style_stack: vec![styles.body],
        link_stack: Vec::new(),
        list_stack: Vec::new(),
        in_blockquote: false,
        blockquote_prefix_emitted: false,
        table: None,
        options,
        pending_code_lang: None,
        in_code_block: false,
        code_buffer: String::new(),
    };
    for event in parser {
        writer.on_event(event);
    }
    writer.finish_paragraph();
    if writer.in_code_block {
        writer.flush_code_block();
    }
    writer.lines
}

struct Writer<'a> {
    styles: &'a MarkdownStyles,
    highlighter: &'a dyn Highlighter,
    streaming_code: &'a mut StreamingCodeState,
    lines: Vec<StyledLine>,
    current: StyledLine,
    style_stack: Vec<TextStyle>,
    link_stack: Vec<Option<String>>,
    list_stack: Vec<ListFrame>,
    in_blockquote: bool,
    blockquote_prefix_emitted: bool,
    table: Option<TableBuilder>,
    options: &'a MarkdownOptions,
    pending_code_lang: Option<String>,
    in_code_block: bool,
    code_buffer: String,
}

struct ListFrame {
    ordered: bool,
    next_index: usize,
    depth: usize,
}

struct TableBuilder {
    alignments: Vec<Alignment>,
    header: Vec<Vec<String>>,
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: String,
    in_header: bool,
}

impl Writer<'_> {
    fn on_event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(end) => self.end_tag(end),
            Event::Text(text) => {
                if self.in_code_block {
                    self.code_buffer.push_str(&text);
                } else if self.table.is_some() {
                    self.table_mut().current_cell.push_str(&text);
                } else {
                    self.push_text(text.as_ref());
                }
            }
            Event::Code(text) => self.push_text_with_style(text.as_ref(), self.styles.code_inline),
            Event::Html(html) | Event::InlineHtml(html) => self.push_text(html.as_ref()),
            Event::SoftBreak => self.push_text(" "),
            Event::HardBreak => self.finish_paragraph(),
            Event::Rule => {
                self.finish_paragraph();
                let rule = "─".repeat(40);
                self.lines.push(StyledLine::raw(rule));
            }
            Event::TaskListMarker(checked) => {
                let mark = if checked { "[x] " } else { "[ ] " };
                self.push_text_with_style(mark, self.styles.list_marker);
            }
            Event::FootnoteReference(_) | Event::InlineMath(_) | Event::DisplayMath(_) => {}
        }
    }

    fn start_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Heading { level, .. } => {
                self.finish_paragraph();
                let style = match level {
                    pulldown_cmark::HeadingLevel::H1 => self.styles.h1,
                    pulldown_cmark::HeadingLevel::H2 => self.styles.h2,
                    pulldown_cmark::HeadingLevel::H3 => self.styles.h3,
                    pulldown_cmark::HeadingLevel::H4 => self.styles.h4,
                    pulldown_cmark::HeadingLevel::H5 => self.styles.h5,
                    pulldown_cmark::HeadingLevel::H6 => self.styles.h6,
                };
                self.style_stack.push(style);
            }
            Tag::BlockQuote(_) => {
                self.finish_paragraph();
                self.in_blockquote = true;
                self.blockquote_prefix_emitted = false;
                self.style_stack.push(self.styles.blockquote);
            }
            Tag::CodeBlock(kind) => {
                self.finish_paragraph();
                self.in_code_block = true;
                self.code_buffer.clear();
                self.pending_code_lang = match kind {
                    CodeBlockKind::Fenced(info) => {
                        let lang = info.to_string();
                        if lang.is_empty() { None } else { Some(lang) }
                    }
                    CodeBlockKind::Indented => None,
                };
            }
            Tag::List(start) => {
                self.list_stack.push(ListFrame {
                    ordered: start.is_some(),
                    next_index: start.map_or(1, |n| usize::try_from(n).unwrap_or(1)),
                    depth: self.list_stack.len(),
                });
            }
            Tag::Item => {
                self.finish_paragraph();
                let (indent, marker) = {
                    let frame = self.list_stack.last().expect("item outside list");
                    let indent = "  ".repeat(frame.depth);
                    let marker = if frame.ordered {
                        let n = frame.next_index;
                        format!("{n}. ")
                    } else {
                        "• ".to_string()
                    };
                    (indent, marker)
                };
                self.push_text_with_style(format!("{indent}{marker}"), self.styles.list_marker);
                if let Some(frame) = self.list_stack.last_mut() {
                    if frame.ordered {
                        frame.next_index += 1;
                    }
                }
            }
            Tag::Emphasis => self.style_stack.push(self.styles.emphasis),
            Tag::Strong => self.style_stack.push(self.styles.strong),
            Tag::Strikethrough => self.style_stack.push(self.styles.strikethrough),
            Tag::Link { dest_url, .. } => {
                self.link_stack.push(Some(dest_url.to_string()));
                self.style_stack.push(self.styles.link);
            }
            Tag::Table(alignments) => {
                self.finish_paragraph();
                self.table = Some(TableBuilder {
                    alignments,
                    header: Vec::new(),
                    rows: Vec::new(),
                    current_row: Vec::new(),
                    current_cell: String::new(),
                    in_header: false,
                });
            }
            Tag::TableHead => {
                if let Some(t) = &mut self.table {
                    t.in_header = true;
                    t.current_row.clear();
                }
            }
            Tag::TableRow => {
                if let Some(t) = &mut self.table {
                    t.current_row.clear();
                    t.current_cell.clear();
                }
            }
            Tag::TableCell => {
                if let Some(t) = &mut self.table {
                    t.current_cell.clear();
                }
            }
            _ => {}
        }
    }

    fn end_tag(&mut self, end: TagEnd) {
        match end {
            TagEnd::Paragraph | TagEnd::Item => self.finish_paragraph(),
            TagEnd::Heading(_) => {
                self.finish_paragraph();
                self.style_stack.pop();
            }
            TagEnd::BlockQuote(_) => {
                self.finish_paragraph();
                self.in_blockquote = false;
                self.blockquote_prefix_emitted = false;
                self.style_stack.pop();
            }
            TagEnd::CodeBlock => {
                self.flush_code_block();
                self.in_code_block = false;
            }
            TagEnd::List(_) => {
                self.list_stack.pop();
                self.finish_paragraph();
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => {
                self.style_stack.pop();
            }
            TagEnd::Link => {
                self.link_stack.pop();
                self.style_stack.pop();
            }
            TagEnd::Table => {
                if let Some(table) = self.table.take() {
                    self.render_table(&table);
                }
            }
            TagEnd::TableHead => {
                if let Some(t) = &mut self.table {
                    t.in_header = false;
                }
            }
            TagEnd::TableRow => {
                if let Some(t) = &mut self.table {
                    if t.in_header {
                        t.header.push(t.current_row.clone());
                    } else {
                        t.rows.push(t.current_row.clone());
                    }
                }
            }
            TagEnd::TableCell => {
                if let Some(t) = &mut self.table {
                    t.current_row.push(t.current_cell.clone());
                    t.current_cell.clear();
                }
            }
            _ => {}
        }
    }

    fn table_mut(&mut self) -> &mut TableBuilder {
        self.table.as_mut().expect("table cell outside table")
    }

    fn flush_code_block(&mut self) {
        let lang_name = self
            .pending_code_lang
            .as_deref()
            .or(self.options.default_code_language.as_deref())
            .and_then(|n| self.highlighter.language_for_name(n))
            .map(|l| l.0)
            .or_else(|| self.streaming_code.lang.clone());
        self.streaming_code.lang.clone_from(&lang_name);

        let body = self.code_buffer.trim_end_matches('\n');
        let source_lines: Vec<&str> = if body.is_empty() {
            vec![""]
        } else {
            body.split('\n').collect()
        };

        if self.streaming_code.line_states.len() == source_lines.len() {
            for (idx, line) in source_lines.iter().enumerate() {
                if self.streaming_code.lines.get(idx).map(String::as_str) != Some(line) {
                    self.streaming_code.lines[idx] = (*line).to_string();
                    let lang = lang_name
                        .as_deref()
                        .and_then(|n| self.highlighter.language_for_name(n))
                        .unwrap_or_else(|| crate::highlight::LangId("Plain Text".to_string()));
                    self.streaming_code.line_states[idx] = self.highlighter.new_state(lang);
                }
            }
            self.streaming_code.lines.truncate(source_lines.len());
            if self.streaming_code.line_states.len() > source_lines.len() {
                self.streaming_code.line_states.truncate(source_lines.len());
            }
        } else {
            let lang = lang_name
                .as_deref()
                .and_then(|n| self.highlighter.language_for_name(n))
                .unwrap_or_else(|| crate::highlight::LangId("Plain Text".to_string()));
            self.streaming_code.line_states = source_lines
                .iter()
                .map(|_| self.highlighter.new_state(lang.clone()))
                .collect();
            self.streaming_code.lines = source_lines.iter().map(ToString::to_string).collect();
        }

        for (idx, line) in source_lines.iter().enumerate() {
            let state = &mut self.streaming_code.line_states[idx];
            let spans = self.highlighter.highlight_line(state, line);
            let mut styled = StyledLine::new();
            if spans.is_empty() {
                styled.push(StyledSpan::styled(*line, self.styles.code_inline));
            } else {
                for span in spans {
                    styled.push(span);
                }
            }
            self.lines.push(styled);
        }
        self.code_buffer.clear();
        self.pending_code_lang = None;
    }

    fn render_table(&mut self, table: &TableBuilder) {
        let col_count = table
            .alignments
            .len()
            .max(table.header.first().map_or(0, std::vec::Vec::len))
            .max(table.rows.iter().map(std::vec::Vec::len).max().unwrap_or(0));
        if col_count == 0 {
            return;
        }
        let mut widths = vec![1usize; col_count];
        for row in table.header.iter().chain(table.rows.iter()) {
            for (i, cell) in row.iter().enumerate() {
                if i < col_count {
                    widths[i] = widths[i].max(cell.width());
                }
            }
        }
        let render_row = |cells: &[String]| -> StyledLine {
            let mut line = StyledLine::new();
            for (i, col_w) in widths.iter().enumerate().take(col_count) {
                let cell = cells.get(i).map_or("", String::as_str);
                let padded = format!(" {:width$} ", cell, width = *col_w);
                line.push(StyledSpan::styled(padded, self.styles.body));
                if i + 1 < col_count {
                    line.push(StyledSpan::styled("│", self.styles.table_border));
                }
            }
            line
        };
        if let Some(header) = table.header.last() {
            self.lines.push(render_row(header));
            let sep = widths
                .iter()
                .map(|w| "─".repeat(w + 2))
                .collect::<Vec<_>>()
                .join("┼");
            self.lines.push(StyledLine::raw(sep));
        }
        for row in &table.rows {
            self.lines.push(render_row(row));
        }
        self.finish_paragraph();
    }

    fn current_style(&self) -> TextStyle {
        self.style_stack.last().copied().unwrap_or(self.styles.body)
    }

    fn push_text(&mut self, text: &str) {
        self.push_text_with_style(text, self.current_style());
    }

    fn push_text_with_style(&mut self, text: impl AsRef<str>, style: TextStyle) {
        if self.in_blockquote && !self.blockquote_prefix_emitted {
            self.blockquote_prefix_emitted = true;
            self.current
                .push(StyledSpan::styled("│ ", self.styles.table_border));
        }
        let span = if let Some(dest) = self.link_stack.last().and_then(|l| l.as_ref()) {
            StyledSpan::link(text.as_ref(), style, dest.clone())
        } else {
            StyledSpan::styled(text.as_ref(), style)
        };
        self.current.push(span);
    }

    fn finish_paragraph(&mut self) {
        if self.current.spans.is_empty() {
            return;
        }
        self.lines.push(self.current.clone());
        self.current = StyledLine::new();
        self.blockquote_prefix_emitted = false;
    }
}

/// Canonical string for tests: plain text plus style/link markers per line.
pub fn lines_signature(lines: &[StyledLine]) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    for line in lines {
        for span in &line.spans {
            let _ = write!(
                out,
                "[{}|fg={:?}|link={}]",
                span.text,
                span.style.fg,
                span.link.as_deref().unwrap_or("")
            );
        }
        out.push('\n');
    }
    out
}
