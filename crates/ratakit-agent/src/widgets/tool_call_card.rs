//! Tool invocation card for transcript rows (see design §6.2 / §7.4).

use std::fmt::Write;

use ratakit_core::{GlyphSet, Motion, MotionMode, Revisioned, SemanticColor, Theme};
use ratakit_text::{DiffModel, Highlighter, wrap_lines};
use ratakit_widgets::{CodeBlock, CodeBlockState, DiffMode, DiffView, DiffViewState, Spinner};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Modifier, Style};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::paragraph::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::state::{ToolCallCell, ToolKind, ToolStatus};
use ratatui_core::style::Color;

/// Default head/tail line counts when truncating long tool output (Codex-style).
pub const TOOL_OUTPUT_HEAD_LINES: usize = 3;
pub const TOOL_OUTPUT_TAIL_LINES: usize = 2;

#[derive(Clone, Copy, Debug)]
struct OutputTruncation {
    head: usize,
    tail: usize,
}

impl OutputTruncation {
    fn new(head: usize, tail: usize) -> Self {
        Self { head, tail }
    }
}

struct RowPaintCtx<'a> {
    theme: &'a Theme,
    highlighter: &'a dyn Highlighter,
    aux: &'a mut ToolCallCardAux,
    skip_top: u16,
    truncation: OutputTruncation,
}

/// Per-transcript auxiliary state for tool-call rows (spinner motion, nested widgets).
#[derive(Debug, Default)]
pub struct ToolCallCardAux {
    motion: Motion,
    output_code: CodeBlockState,
    diff_view: DiffViewState,
    cached_revision: u64,
    cached_width: u16,
    cached_height: u16,
}

/// Renders a [`ToolCallCell`] with status, I/O summary, truncated output, and optional diff.
pub struct ToolCallCard<'a> {
    theme: &'a Theme,
    highlighter: &'a dyn Highlighter,
    cell: &'a ToolCallCell,
    head_lines: usize,
    tail_lines: usize,
}

impl<'a> ToolCallCard<'a> {
    /// Create a card bound to a transcript tool-call cell.
    pub fn new(theme: &'a Theme, highlighter: &'a dyn Highlighter, cell: &'a ToolCallCell) -> Self {
        Self {
            theme,
            highlighter,
            cell,
            head_lines: TOOL_OUTPUT_HEAD_LINES,
            tail_lines: TOOL_OUTPUT_TAIL_LINES,
        }
    }

    /// Override head/tail truncation (for tests).
    pub fn truncation(mut self, head: usize, tail: usize) -> Self {
        self.head_lines = head;
        self.tail_lines = tail;
        self
    }

    /// Desired height in rows for the given width.
    pub fn measure(&self, width: u16, aux: &mut ToolCallCardAux) -> u16 {
        measure_tool_call_card(
            self.cell,
            width,
            self.theme,
            self.highlighter,
            aux,
            self.head_lines,
            self.tail_lines,
        )
    }

    /// Paint the card into `area`, skipping the first `skip_top` visual rows.
    pub fn paint(&self, area: Rect, buf: &mut Buffer, aux: &mut ToolCallCardAux, skip_top: u16) {
        render_tool_call_card(
            self.cell,
            area,
            buf,
            self.theme,
            self.highlighter,
            aux,
            skip_top,
            self.head_lines,
            self.tail_lines,
        );
    }
}

impl StatefulWidget for ToolCallCard<'_> {
    type State = ToolCallCardAux;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        render_tool_call_card(
            self.cell,
            area,
            buf,
            self.theme,
            self.highlighter,
            state,
            0,
            self.head_lines,
            self.tail_lines,
        );
    }
}

/// Height in terminal rows for a tool-call transcript cell.
pub fn measure_tool_call_card(
    cell: &ToolCallCell,
    width: u16,
    theme: &Theme,
    highlighter: &dyn Highlighter,
    aux: &mut ToolCallCardAux,
    head_lines: usize,
    tail_lines: usize,
) -> u16 {
    let truncation = OutputTruncation::new(head_lines, tail_lines);
    let rev = cell_revision(cell, aux);
    if aux.cached_width == width && aux.cached_revision == rev && aux.cached_height > 0 {
        return aux.cached_height;
    }
    let rows = build_layout(cell, width, theme, highlighter, aux, truncation);
    let height = rows.iter().map(|r| r.height).sum();
    aux.cached_width = width;
    aux.cached_revision = rev;
    aux.cached_height = height;
    height
}

/// Paint a tool-call cell (see [`ToolCallCard::paint`]).
#[allow(clippy::too_many_arguments)]
pub fn render_tool_call_card(
    cell: &ToolCallCell,
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    highlighter: &dyn Highlighter,
    aux: &mut ToolCallCardAux,
    skip_top: u16,
    head_lines: usize,
    tail_lines: usize,
) {
    if area.is_empty() {
        return;
    }
    let truncation = OutputTruncation::new(head_lines, tail_lines);
    let rows = build_layout(cell, area.width, theme, highlighter, aux, truncation);
    let mut y = area.y;
    let mut remaining = area.height;
    let mut skipped = skip_top;

    for row in rows {
        if skipped >= row.height {
            skipped -= row.height;
            continue;
        }
        if remaining == 0 {
            break;
        }
        let row_skip = skipped;
        skipped = 0;
        let h = row.height.min(remaining);
        let row_area = Rect {
            x: area.x,
            y,
            width: area.width,
            height: h,
        };
        let mut ctx = RowPaintCtx {
            theme,
            highlighter,
            aux,
            skip_top: row_skip,
            truncation,
        };
        row.draw(cell, row_area, buf, &mut ctx);
        y += h;
        remaining -= h;
    }
}

/// Hook for the transcript virtual list (milestone A2).
pub struct ToolCallTranscriptRenderer<'a> {
    theme: &'a Theme,
    highlighter: &'a dyn Highlighter,
}

impl<'a> ToolCallTranscriptRenderer<'a> {
    pub fn new(theme: &'a Theme, highlighter: &'a dyn Highlighter) -> Self {
        Self { theme, highlighter }
    }

    pub fn row_height(&self, cell: &ToolCallCell, width: u16, aux: &mut ToolCallCardAux) -> u16 {
        measure_tool_call_card(
            cell,
            width,
            self.theme,
            self.highlighter,
            aux,
            TOOL_OUTPUT_HEAD_LINES,
            TOOL_OUTPUT_TAIL_LINES,
        )
    }

    pub fn render_row(
        &self,
        cell: &ToolCallCell,
        area: Rect,
        buf: &mut Buffer,
        aux: &mut ToolCallCardAux,
        skip_top: u16,
    ) {
        render_tool_call_card(
            cell,
            area,
            buf,
            self.theme,
            self.highlighter,
            aux,
            skip_top,
            TOOL_OUTPUT_HEAD_LINES,
            TOOL_OUTPUT_TAIL_LINES,
        );
    }
}

fn cell_revision(cell: &ToolCallCell, aux: &ToolCallCardAux) -> u64 {
    let folded = u64::from(cell.output_folded());
    let out_len = u64::try_from(cell.output.len()).unwrap_or(u64::MAX);
    let diff = u64::from(cell.diff.is_some());
    folded ^ (out_len << 1) ^ (diff << 2) ^ aux.motion.revision() ^ aux.output_code.revision()
}

#[derive(Clone, Copy)]
enum LayoutRowKind {
    Header,
    Title,
    Locations,
    InputHeader,
    InputBody,
    OutputHeader,
    OutputFolded,
    OutputCode,
    Diff,
}

struct LayoutRow {
    kind: LayoutRowKind,
    height: u16,
}

impl LayoutRow {
    fn draw(&self, cell: &ToolCallCell, area: Rect, buf: &mut Buffer, ctx: &mut RowPaintCtx<'_>) {
        match self.kind {
            LayoutRowKind::Header => draw_header(cell, area, buf, ctx.theme, ctx.aux),
            LayoutRowKind::Title => {
                draw_wrapped_text(
                    cell.title(),
                    area,
                    buf,
                    ctx.theme,
                    fg_style(ctx.theme),
                    ctx.skip_top,
                );
            }
            LayoutRowKind::Locations => draw_locations(cell, area, buf, ctx.theme),
            LayoutRowKind::InputHeader => draw_label("input", area, buf, ctx.theme),
            LayoutRowKind::InputBody => {
                draw_wrapped_text(
                    cell.input_text(),
                    area,
                    buf,
                    ctx.theme,
                    muted_style(ctx.theme),
                    ctx.skip_top,
                );
            }
            LayoutRowKind::OutputHeader => draw_label("output", area, buf, ctx.theme),
            LayoutRowKind::OutputFolded => draw_output_folded(cell, area, buf, ctx.theme),
            LayoutRowKind::OutputCode => draw_output_code(cell, area, buf, ctx),
            LayoutRowKind::Diff => {
                draw_diff(
                    cell,
                    area,
                    buf,
                    ctx.theme,
                    ctx.highlighter,
                    ctx.aux,
                    ctx.skip_top,
                );
            }
        }
    }
}

fn build_layout(
    cell: &ToolCallCell,
    width: u16,
    _theme: &Theme,
    highlighter: &dyn Highlighter,
    aux: &mut ToolCallCardAux,
    truncation: OutputTruncation,
) -> Vec<LayoutRow> {
    let w = width.max(1);
    let mut rows = vec![LayoutRow {
        kind: LayoutRowKind::Header,
        height: 1,
    }];

    if !cell.title().is_empty() {
        rows.push(LayoutRow {
            kind: LayoutRowKind::Title,
            height: wrapped_row_count(cell.title(), w),
        });
    }

    if !cell.locations().is_empty() {
        rows.push(LayoutRow {
            kind: LayoutRowKind::Locations,
            height: 1,
        });
    }

    if !cell.input_text().is_empty() {
        rows.push(LayoutRow {
            kind: LayoutRowKind::InputHeader,
            height: 1,
        });
        rows.push(LayoutRow {
            kind: LayoutRowKind::InputBody,
            height: wrapped_row_count(cell.input_text(), w.saturating_sub(2)),
        });
    }

    if show_output_section(cell) {
        rows.push(LayoutRow {
            kind: LayoutRowKind::OutputHeader,
            height: 1,
        });
        if cell.output_folded() {
            rows.push(LayoutRow {
                kind: LayoutRowKind::OutputFolded,
                height: 1,
            });
        } else {
            let display = output_display_text(cell, truncation);
            sync_output_code(aux, &display, highlighter, cell);
            rows.push(LayoutRow {
                kind: LayoutRowKind::OutputCode,
                height: output_code_height(aux, w, highlighter),
            });
        }
    }

    if let Some(model) = cell.inline_diff() {
        rows.push(LayoutRow {
            kind: LayoutRowKind::Diff,
            height: diff_block_height(model),
        });
    }

    rows
}

fn show_output_section(cell: &ToolCallCell) -> bool {
    !cell.output_text().is_empty()
}

fn wrapped_row_count(text: &str, width: u16) -> u16 {
    if text.is_empty() {
        return 1;
    }
    let w = width.max(1);
    text.lines()
        .map(|line| wrap_lines(line, w).len().max(1))
        .map(|n| u16::try_from(n).unwrap_or(u16::MAX))
        .sum()
}

fn draw_header(
    cell: &ToolCallCell,
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    aux: &mut ToolCallCardAux,
) {
    if area.is_empty() {
        return;
    }
    aux.motion.set_mode(theme.motion);

    let status_style = status_style(cell.status(), theme);
    let kind_label = kind_label(cell.kind());
    let kind_icon = kind_icon(cell.kind(), theme.glyphs);

    let mut spans = Vec::new();

    if cell.status() == ToolStatus::InProgress {
        let spin_w = spinner_column_width(theme, aux).min(area.width);
        let spin_area = Rect {
            x: area.x,
            y: area.y,
            width: spin_w,
            height: 1,
        };
        Spinner::new(theme)
            .label("")
            .render(spin_area, buf, &mut aux.motion);
        let rest = Rect {
            x: area.x + spin_w,
            y: area.y,
            width: area.width.saturating_sub(spin_w),
            height: 1,
        };
        if rest.width > 0 {
            let line = Line::from(vec![
                Span::styled(format!("{kind_icon} {kind_label}  "), accent_style(theme)),
                Span::styled(status_name(cell.status()), status_style),
            ]);
            Paragraph::new(line).render(rest, buf);
        }
        return;
    }

    let status_glyph = status_glyph(cell.status(), theme.glyphs);
    spans.push(Span::styled(
        format!("{status_glyph} "),
        status_style.add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(
        format!("{kind_icon} {kind_label}  "),
        accent_style(theme),
    ));
    spans.push(Span::styled(status_name(cell.status()), status_style));
    Paragraph::new(Line::from(spans)).render(area, buf);
}

fn draw_wrapped_text(
    text: &str,
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    style: Style,
    skip_top: u16,
) {
    if area.is_empty() {
        return;
    }
    let w = area.width.max(1);
    let mut visual: Vec<Line<'static>> = Vec::new();
    for line in text.lines() {
        let wrapped = wrap_lines(line, w);
        if wrapped.is_empty() {
            visual.push(Line::from(Span::styled(" ", style)));
        } else {
            for wl in wrapped {
                visual.push(Line::from(Span::styled(wl.text, style)));
            }
        }
    }
    if visual.is_empty() {
        visual.push(Line::from(Span::styled(" ", muted_style(theme))));
    }
    let skip = usize::from(skip_top).min(visual.len());
    let visible = visual
        .into_iter()
        .skip(skip)
        .take(area.height as usize)
        .collect::<Vec<_>>();
    Paragraph::new(visible).render(area, buf);
}

fn draw_locations(cell: &ToolCallCell, area: Rect, buf: &mut Buffer, theme: &Theme) {
    let joined = cell
        .locations()
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let text = truncate_to_width(&joined, area.width);
    Paragraph::new(Line::from(Span::styled(text, muted_style(theme)))).render(area, buf);
}

fn draw_label(label: &str, area: Rect, buf: &mut Buffer, theme: &Theme) {
    Paragraph::new(Line::from(Span::styled(
        format!("{label}:"),
        fg_style(theme).add_modifier(Modifier::BOLD),
    )))
    .render(area, buf);
}

fn draw_output_folded(cell: &ToolCallCell, area: Rect, buf: &mut Buffer, theme: &Theme) {
    let n = cell.output_line_count();
    let glyph = match theme.glyphs {
        GlyphSet::Unicode => "▸",
        GlyphSet::Ascii => ">",
    };
    let text = if n == 0 {
        format!("{glyph} output (folded)")
    } else {
        format!("{glyph} output ({n} lines, folded)")
    };
    Paragraph::new(Line::from(Span::styled(text, muted_style(theme)))).render(area, buf);
}

fn output_display_text(cell: &ToolCallCell, truncation: OutputTruncation) -> String {
    let head = truncation.head;
    let tail = truncation.tail;
    let text = cell.output_text();
    if text.is_empty() {
        return String::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= head + tail {
        return text.to_string();
    }
    let omitted = lines.len() - head - tail;
    let mut out = String::new();
    for line in lines.iter().take(head) {
        out.push_str(line);
        out.push('\n');
    }
    let _ = writeln!(out, "… {omitted} more lines");
    for line in lines.iter().skip(lines.len() - tail) {
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn sync_output_code(
    aux: &mut ToolCallCardAux,
    display: &str,
    highlighter: &dyn Highlighter,
    cell: &ToolCallCell,
) {
    if aux.output_code.source() != display {
        aux.output_code.set_source(display);
    }
    if let Some(path) = cell.locations().first() {
        aux.output_code.detect_language_from_path(path, highlighter);
    }
}

fn output_code_height(aux: &ToolCallCardAux, width: u16, _highlighter: &dyn Highlighter) -> u16 {
    let w = width.max(1);
    let lines = aux.output_code.source().lines().collect::<Vec<_>>();
    if lines.is_empty() {
        return 1;
    }
    let mut total = 0u16;
    for line in lines {
        let wrapped = wrap_lines(line, w);
        total += u16::try_from(wrapped.len().max(1)).unwrap_or(u16::MAX);
    }
    total.max(1)
}

fn draw_output_code(cell: &ToolCallCell, area: Rect, buf: &mut Buffer, ctx: &mut RowPaintCtx<'_>) {
    let display = output_display_text(cell, ctx.truncation);
    sync_output_code(ctx.aux, &display, ctx.highlighter, cell);
    ctx.aux
        .output_code
        .set_scroll_row(usize::from(ctx.skip_top));
    CodeBlock::new(ctx.theme, ctx.highlighter)
        .line_numbers(false)
        .render(area, buf, &mut ctx.aux.output_code);
}

fn diff_block_height(model: &DiffModel) -> u16 {
    let mut rows = 0u16;
    for file in &model.files {
        rows += 1;
        for hunk in &file.hunks {
            rows += 1;
            if hunk.collapsed {
                rows += 1;
            } else {
                let add = u16::try_from(hunk.lines.len()).unwrap_or(u16::MAX);
                rows = rows.saturating_add(add);
            }
        }
    }
    rows.max(1)
}

fn draw_diff(
    cell: &ToolCallCell,
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    highlighter: &dyn Highlighter,
    aux: &mut ToolCallCardAux,
    skip_top: u16,
) {
    let model = cell
        .inline_diff()
        .expect("layout only adds diff when present");
    aux.diff_view
        .list_mut()
        .scroll_mut()
        .scroll_to_offset(usize::from(skip_top));
    DiffView::new(model, theme, highlighter)
        .mode(DiffMode::Unified)
        .line_numbers(false)
        .scrollbar(false)
        .render(area, buf, &mut aux.diff_view);
}

fn truncate_to_width(text: &str, width: u16) -> String {
    let w = width.max(1) as usize;
    if text.width() <= w {
        return text.to_string();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw + 1 > w {
            out.push('…');
            break;
        }
        out.push(ch);
        used += cw;
    }
    out
}

fn kind_label(kind: ToolKind) -> &'static str {
    match kind {
        ToolKind::Read => "read",
        ToolKind::Edit => "edit",
        ToolKind::Execute => "execute",
        ToolKind::Search => "search",
        ToolKind::Fetch => "fetch",
        ToolKind::Other => "other",
    }
}

fn kind_icon(kind: ToolKind, glyphs: GlyphSet) -> &'static str {
    match (kind, glyphs) {
        (ToolKind::Read, GlyphSet::Unicode) => "◧",
        (ToolKind::Read, GlyphSet::Ascii) => "R",
        (ToolKind::Edit, GlyphSet::Unicode) => "✎",
        (ToolKind::Edit, GlyphSet::Ascii) => "E",
        (ToolKind::Execute, GlyphSet::Unicode) => "▶",
        (ToolKind::Execute, GlyphSet::Ascii) => ">",
        (ToolKind::Search, GlyphSet::Unicode) => "⌕",
        (ToolKind::Search, GlyphSet::Ascii) => "?",
        (ToolKind::Fetch, GlyphSet::Unicode) => "↓",
        (ToolKind::Fetch, GlyphSet::Ascii) => "v",
        (ToolKind::Other, GlyphSet::Unicode) => "•",
        (ToolKind::Other, GlyphSet::Ascii) => "*",
    }
}

fn status_name(status: ToolStatus) -> &'static str {
    match status {
        ToolStatus::Pending => "pending",
        ToolStatus::InProgress => "running",
        ToolStatus::Completed => "done",
        ToolStatus::Failed => "failed",
        ToolStatus::Cancelled => "cancelled",
    }
}

fn status_glyph(status: ToolStatus, glyphs: GlyphSet) -> &'static str {
    match (status, glyphs) {
        (ToolStatus::Pending, GlyphSet::Unicode) => "○",
        (ToolStatus::Pending, GlyphSet::Ascii) => "o",
        (ToolStatus::InProgress, GlyphSet::Unicode) => "…",
        (ToolStatus::InProgress, GlyphSet::Ascii) => "..",
        (ToolStatus::Completed, GlyphSet::Unicode) => "✓",
        (ToolStatus::Completed, GlyphSet::Ascii) => "ok",
        (ToolStatus::Failed, GlyphSet::Unicode) => "✗",
        (ToolStatus::Failed, GlyphSet::Ascii) => "!!",
        (ToolStatus::Cancelled, GlyphSet::Unicode) => "⊘",
        (ToolStatus::Cancelled, GlyphSet::Ascii) => "x",
    }
}

fn status_style(status: ToolStatus, theme: &Theme) -> Style {
    let color = match status {
        ToolStatus::Pending => theme.palette.muted,
        ToolStatus::InProgress => theme.palette.accent,
        ToolStatus::Completed => theme.palette.success,
        ToolStatus::Failed => theme.palette.error,
        ToolStatus::Cancelled => theme.palette.warning,
    };
    Style::new().fg(semantic_color(color))
}

fn semantic_color(color: SemanticColor) -> Color {
    match color {
        SemanticColor::Rgb(rgb) => Color::Rgb(rgb.r, rgb.g, rgb.b),
        SemanticColor::Index(i) => Color::Indexed(i),
    }
}

fn fg_style(theme: &Theme) -> Style {
    Style::new().fg(semantic_color(theme.palette.foreground))
}

fn muted_style(theme: &Theme) -> Style {
    Style::new().fg(semantic_color(theme.palette.muted))
}

fn accent_style(theme: &Theme) -> Style {
    Style::new().fg(semantic_color(theme.palette.accent))
}

fn spinner_column_width(theme: &Theme, aux: &ToolCallCardAux) -> u16 {
    if theme.motion == MotionMode::Reduced {
        let w = aux.motion.reduced_label().width();
        u16::try_from(w + 1).unwrap_or(10)
    } else {
        2
    }
}
