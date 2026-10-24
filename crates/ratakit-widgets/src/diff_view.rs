//! Unified and side-by-side diff viewer.

use ratakit_core::{ItemHeights, ListState, Revisioned, Theme};
use ratakit_text::{
    ChangeTag, DiffLine, DiffModel, DiffSide, FileDiff, Highlighter, LangId, StyledLine,
    StyledSpan, TextStyle, display_path, path_for_side,
};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use unicode_width::UnicodeWidthStr;

use crate::styled_convert::{styled_line_to_ratatui_owned, text_style_to_ratatui};
use crate::virtual_list::{RowRenderer, VirtualList};

/// Layout mode for [`DiffView`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffMode {
    Unified,
    SideBySide,
    /// Pick [`DiffMode::SideBySide`] when `width >= min_width`, otherwise [`DiffMode::Unified`].
    Auto {
        min_width: u16,
    },
}

impl Default for DiffMode {
    fn default() -> Self {
        Self::Unified
    }
}

/// Scroll and highlight state for [`DiffView`].
#[derive(Debug, Default)]
pub struct DiffViewState {
    list: ListState,
    revision: u64,
}

impl DiffViewState {
    /// New state with follow-tail scrolling enabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Underlying virtual list state (scroll offset, selection).
    pub fn list(&self) -> &ListState {
        &self.list
    }

    /// Mutable list state.
    pub fn list_mut(&mut self) -> &mut ListState {
        &mut self.list
    }

    /// Toggle collapse on a hunk in the model.
    pub fn toggle_hunk(&mut self, model: &mut DiffModel, file_index: usize, hunk_index: usize) {
        model.toggle_hunk_collapsed(file_index, hunk_index);
        self.revision += 1;
    }
}

impl Revisioned for DiffViewState {
    fn revision(&self) -> u64 {
        self.revision.max(self.list.revision())
    }
}

/// Renders a [`DiffModel`] with syntax highlighting and virtualized scrolling.
pub struct DiffView<'a> {
    model: &'a DiffModel,
    mode: DiffMode,
    theme: &'a Theme,
    highlighter: &'a dyn Highlighter,
    show_line_numbers: bool,
    show_scrollbar: bool,
}

impl<'a> DiffView<'a> {
    /// Create a diff view with default unified layout.
    pub fn new(model: &'a DiffModel, theme: &'a Theme, highlighter: &'a dyn Highlighter) -> Self {
        Self {
            model,
            mode: DiffMode::default(),
            theme,
            highlighter,
            show_line_numbers: true,
            show_scrollbar: true,
        }
    }

    pub fn mode(mut self, mode: DiffMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn line_numbers(mut self, on: bool) -> Self {
        self.show_line_numbers = on;
        self
    }

    pub fn scrollbar(mut self, on: bool) -> Self {
        self.show_scrollbar = on;
        self
    }
}

impl StatefulWidget for DiffView<'_> {
    type State = DiffViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.is_empty() {
            return;
        }
        let mode = effective_mode(self.mode, area.width);
        let layout = DiffLayout::build(self.model, mode);
        let rows = DiffRows {
            model: self.model,
            layout: &layout,
            theme: self.theme,
            highlighter: self.highlighter,
            show_line_numbers: self.show_line_numbers,
            view_revision: state.revision(),
            width: area.width,
        };
        let list = VirtualList::new(&rows, self.theme).scrollbar(self.show_scrollbar);
        list.render(area, buf, &mut state.list);
    }
}

fn effective_mode(mode: DiffMode, width: u16) -> EffectiveMode {
    match mode {
        DiffMode::Unified => EffectiveMode::Unified,
        DiffMode::SideBySide => EffectiveMode::SideBySide,
        DiffMode::Auto { min_width } => {
            if width >= min_width {
                EffectiveMode::SideBySide
            } else {
                EffectiveMode::Unified
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectiveMode {
    Unified,
    SideBySide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowKind {
    FileHeader,
    HunkHeader,
    Collapsed,
    UnifiedLine,
    SplitLine,
}

#[derive(Debug, Clone)]
struct DiffRowMeta {
    kind: RowKind,
    file_index: usize,
    hunk_index: usize,
    line_index: Option<usize>,
}

struct DiffLayout {
    rows: Vec<DiffRowMeta>,
}

impl DiffLayout {
    fn build(model: &DiffModel, mode: EffectiveMode) -> Self {
        let mut rows = Vec::new();
        for (file_index, file) in model.files.iter().enumerate() {
            rows.push(DiffRowMeta {
                kind: RowKind::FileHeader,
                file_index,
                hunk_index: 0,
                line_index: None,
            });
            for (hunk_index, hunk) in file.hunks.iter().enumerate() {
                rows.push(DiffRowMeta {
                    kind: RowKind::HunkHeader,
                    file_index,
                    hunk_index,
                    line_index: None,
                });
                if hunk.collapsed {
                    rows.push(DiffRowMeta {
                        kind: RowKind::Collapsed,
                        file_index,
                        hunk_index,
                        line_index: None,
                    });
                    continue;
                }
                if mode == EffectiveMode::Unified {
                    for (line_index, _) in hunk.lines.iter().enumerate() {
                        rows.push(DiffRowMeta {
                            kind: RowKind::UnifiedLine,
                            file_index,
                            hunk_index,
                            line_index: Some(line_index),
                        });
                    }
                } else {
                    for (line_index, _) in hunk.lines.iter().enumerate() {
                        rows.push(DiffRowMeta {
                            kind: RowKind::SplitLine,
                            file_index,
                            hunk_index,
                            line_index: Some(line_index),
                        });
                    }
                }
            }
        }
        Self { rows }
    }
}

struct DiffRows<'a> {
    model: &'a DiffModel,
    layout: &'a DiffLayout,
    theme: &'a Theme,
    highlighter: &'a dyn Highlighter,
    show_line_numbers: bool,
    view_revision: u64,
    width: u16,
}

impl ItemHeights for DiffRows<'_> {
    fn len(&self) -> usize {
        self.layout.rows.len()
    }

    fn height(&self, _index: usize, _width: u16) -> u16 {
        1
    }

    fn revision(&self) -> u64 {
        self.view_revision
    }
}

impl RowRenderer for DiffRows<'_> {
    fn render_row(&self, index: usize, area: Rect, buf: &mut Buffer, skip_top: u16) {
        if skip_top > 0 || area.height == 0 {
            return;
        }
        let row = &self.layout.rows[index];
        let file = &self.model.files[row.file_index];
        let rat_line = match row.kind {
            RowKind::FileHeader => file_header_line(file, self.theme),
            RowKind::HunkHeader => {
                let hunk = &file.hunks[row.hunk_index];
                hunk_header_line(hunk, self.theme)
            }
            RowKind::Collapsed => collapsed_line(&file.hunks[row.hunk_index], self.theme),
            RowKind::UnifiedLine => {
                let hunk = &file.hunks[row.hunk_index];
                let line = &hunk.lines[row.line_index.unwrap_or(0)];
                unified_line(
                    row.file_index,
                    line,
                    file,
                    self.theme,
                    self.highlighter,
                    self.show_line_numbers,
                    self.width,
                )
            }
            RowKind::SplitLine => {
                let hunk = &file.hunks[row.hunk_index];
                let line = &hunk.lines[row.line_index.unwrap_or(0)];
                split_line(
                    row.file_index,
                    line,
                    file,
                    self.theme,
                    self.highlighter,
                    self.show_line_numbers,
                    self.width,
                )
            }
        };
        rat_line.render(Rect { height: 1, ..area }, buf);
    }
}

fn file_header_line(file: &FileDiff, theme: &Theme) -> Line<'static> {
    let path = display_path(file).unwrap_or("(unknown)");
    let accent = text_style_to_ratatui(TextStyle::fg(theme.palette.accent));
    Line::from(Span::styled(path.to_string(), accent))
}

fn hunk_header_line(hunk: &ratakit_text::DiffHunk, theme: &Theme) -> Line<'static> {
    let muted = text_style_to_ratatui(TextStyle::fg(theme.palette.muted));
    Line::from(Span::styled(
        format!(
            "@@ -{},{} +{},{} @@",
            hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
        ),
        muted,
    ))
}

fn collapsed_line(hunk: &ratakit_text::DiffHunk, theme: &Theme) -> Line<'static> {
    let muted = text_style_to_ratatui(TextStyle::fg(theme.palette.muted));
    Line::from(Span::styled(
        format!("… {} lines hidden …", hunk.lines.len()),
        muted,
    ))
}

fn unified_line(
    _file_index: usize,
    line: &DiffLine,
    file: &FileDiff,
    theme: &Theme,
    highlighter: &dyn Highlighter,
    show_numbers: bool,
    width: u16,
) -> Line<'static> {
    let marker = match line.tag {
        ChangeTag::Equal => ' ',
        ChangeTag::Delete => '-',
        ChangeTag::Insert => '+',
    };
    let marker_style = diff_tag_style(line.tag, theme);
    let gutter = if show_numbers {
        unified_gutter_width(file)
    } else {
        0
    };
    let code_width = width.saturating_sub(gutter).saturating_sub(2).max(1);

    let side = match line.tag {
        ChangeTag::Delete => DiffSide::Old,
        _ => DiffSide::New,
    };
    let styled = diff_styled_line(line, file, side, theme, highlighter);
    let mut out = Line::default();
    if gutter > 0 {
        let col = (gutter.saturating_sub(1) / 2).max(1) as usize;
        let old_s = line
            .old_line_no
            .map(|n| format!("{n:>col$}"))
            .unwrap_or_else(|| " ".repeat(col));
        let new_s = line
            .new_line_no
            .map(|n| format!("{n:>col$}"))
            .unwrap_or_else(|| " ".repeat(col));
        let nums = format!("{old_s} {new_s} ");
        out.push_span(Span::styled(
            nums,
            text_style_to_ratatui(TextStyle::fg(theme.palette.muted)),
        ));
    }
    out.push_span(Span::styled(marker.to_string(), marker_style));
    out.push_span(Span::raw(" "));
    append_styled_truncated(&mut out, &styled, code_width as usize);
    out
}

fn split_line(
    _file_index: usize,
    line: &DiffLine,
    file: &FileDiff,
    theme: &Theme,
    highlighter: &dyn Highlighter,
    show_numbers: bool,
    width: u16,
) -> Line<'static> {
    let gutter = if show_numbers {
        split_gutter_width(file)
    } else {
        0
    };
    let half = width.saturating_sub(gutter).saturating_sub(1).max(2) / 2;
    let left_width = half;
    let right_width = width
        .saturating_sub(gutter)
        .saturating_sub(1)
        .saturating_sub(half);

    let mut out = Line::default();
    if show_numbers {
        let nums = format!(
            "{:>4} {:>4} ",
            line.old_line_no.map_or(String::new(), |n| n.to_string()),
            line.new_line_no.map_or(String::new(), |n| n.to_string())
        );
        out.push_span(Span::styled(
            nums,
            text_style_to_ratatui(TextStyle::fg(theme.palette.muted)),
        ));
    }

    let left = match line.tag {
        ChangeTag::Insert => Line::from(Span::raw("")),
        _ => styled_to_line(
            diff_styled_line(line, file, DiffSide::Old, theme, highlighter),
            left_width as usize,
        ),
    };
    let right = match line.tag {
        ChangeTag::Delete => Line::from(Span::raw("")),
        _ => styled_to_line(
            diff_styled_line(line, file, DiffSide::New, theme, highlighter),
            right_width as usize,
        ),
    };

    for span in left.spans {
        out.push_span(span);
    }
    out.push_span(Span::styled(
        "│",
        text_style_to_ratatui(TextStyle::fg(theme.palette.border)),
    ));
    for span in right.spans {
        out.push_span(span);
    }
    out
}

fn styled_to_line(styled: StyledLine, max_width: usize) -> Line<'static> {
    let mut out = Line::default();
    append_styled_truncated(&mut out, &styled, max_width);
    pad_line(&mut out, max_width);
    out
}

fn pad_line(line: &mut Line<'static>, width: usize) {
    let used = line.width();
    if used < width {
        line.push_span(Span::raw(" ".repeat(width - used)));
    }
}

fn append_styled_truncated(line: &mut Line<'static>, styled: &StyledLine, max_width: usize) {
    let mut used = 0usize;
    for span in &styled.spans {
        if used >= max_width {
            break;
        }
        let mut single = StyledLine::new();
        single.push(span.clone());
        let rat = styled_line_to_ratatui_owned(&single);
        for s in rat.spans {
            let w = s.content.width();
            if used + w <= max_width {
                line.push_span(s);
                used += w;
            } else {
                let take = max_width - used;
                let content: String = s.content.chars().take(take).collect();
                line.push_span(Span::styled(content, s.style));
                used = max_width;
                break;
            }
        }
    }
}

fn diff_styled_line(
    line: &DiffLine,
    file: &FileDiff,
    side: DiffSide,
    theme: &Theme,
    highlighter: &dyn Highlighter,
) -> StyledLine {
    let lang = language_for(file, side, highlighter);
    let line_no = match side {
        DiffSide::Old => line.old_line_no,
        DiffSide::New => line.new_line_no,
    };
    let syntax = if line_no.is_some() {
        highlight_line(&lang, &line.content, highlighter)
    } else {
        Vec::new()
    };

    if line.intra.iter().any(|s| s.emphasized) {
        let mut out = StyledLine::new();
        for part in &line.intra {
            let style = if part.emphasized {
                diff_emphasis_style(line.tag, theme)
            } else if syntax.is_empty() {
                TextStyle::PLAIN
            } else {
                syntax_style_for_range(&syntax, &line.content, part.text.as_str())
            };
            out.push(StyledSpan::styled(&part.text, style));
        }
        out
    } else if syntax.is_empty() {
        StyledLine::raw(&line.content)
    } else {
        let mut sl = StyledLine::new();
        for s in syntax {
            sl.push(s);
        }
        sl
    }
}

fn syntax_style_for_range(syntax: &[StyledSpan], full: &str, segment: &str) -> TextStyle {
    let start = full.find(segment).unwrap_or(0);
    let end = start + segment.len();
    let mut offset = 0usize;
    for span in syntax {
        let len = span.text.len();
        let span_end = offset + len;
        if offset < end && span_end > start {
            return span.style;
        }
        offset = span_end;
    }
    TextStyle::PLAIN
}

fn highlight_line(lang: &LangId, content: &str, highlighter: &dyn Highlighter) -> Vec<StyledSpan> {
    let mut state = highlighter.new_state(lang.clone());
    highlighter.highlight_line(&mut state, content)
}

fn language_for(file: &FileDiff, side: DiffSide, highlighter: &dyn Highlighter) -> LangId {
    path_for_side(file, side)
        .and_then(|p| highlighter.language_for_path(p))
        .unwrap_or_else(|| LangId("Plain Text".to_string()))
}

fn diff_tag_style(tag: ChangeTag, theme: &Theme) -> Style {
    let color = match tag {
        ChangeTag::Equal => theme.palette.foreground,
        ChangeTag::Delete => theme.palette.error,
        ChangeTag::Insert => theme.palette.success,
    };
    text_style_to_ratatui(TextStyle::fg(color))
}

fn diff_emphasis_style(tag: ChangeTag, theme: &Theme) -> TextStyle {
    let fg = match tag {
        ChangeTag::Equal => theme.palette.foreground,
        ChangeTag::Delete => theme.palette.error,
        ChangeTag::Insert => theme.palette.success,
    };
    TextStyle::fg(fg).with_modifier(ratakit_text::TextModifier::BOLD)
}

fn unified_gutter_width(file: &FileDiff) -> u16 {
    let max_line = file
        .hunks
        .iter()
        .flat_map(|h| h.lines.iter())
        .flat_map(|l| [l.old_line_no, l.new_line_no])
        .flatten()
        .max()
        .unwrap_or(1);
    let digits = max_line.checked_ilog10().map_or(0, |d| d as usize) + 1;
    u16::try_from((digits + 1) * 2 + 1).unwrap_or(12)
}

fn split_gutter_width(_file: &FileDiff) -> u16 {
    11
}
