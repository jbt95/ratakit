//! Plan task list on [`VirtualList`], plus transcript plan-cell helpers.

use ratakit_core::{GlyphSet, ItemHeights, ListState, Theme};
use ratakit_text::wrap_lines;
use ratakit_widgets::{RowRenderer, VirtualList};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::paragraph::Paragraph;

use crate::state::{Plan, PlanEntry, PlanEntryStatus};

use super::style::{accent, fg, muted, success};

const MARKER_COLUMNS: u16 = 2;

/// Virtualized plan task list (side panel or embedded).
pub struct PlanView<'a> {
    plan: &'a Plan,
    theme: &'a Theme,
    show_scrollbar: bool,
}

impl<'a> PlanView<'a> {
    #[must_use]
    pub fn new(plan: &'a Plan, theme: &'a Theme) -> Self {
        Self {
            plan,
            theme,
            show_scrollbar: false,
        }
    }

    #[must_use]
    pub fn scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }
}

impl StatefulWidget for PlanView<'_> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListState) {
        let rows = PlanRows {
            plan: self.plan,
            theme: self.theme,
        };
        let list = VirtualList::new(&rows, self.theme).scrollbar(self.show_scrollbar);
        list.render(area, buf, state);
    }
}

/// Row source for [`VirtualList`] over a [`Plan`].
pub struct PlanRows<'a> {
    plan: &'a Plan,
    theme: &'a Theme,
}

impl ItemHeights for PlanRows<'_> {
    fn len(&self) -> usize {
        self.plan.entries.len()
    }

    fn height(&self, index: usize, width: u16) -> u16 {
        let entry = &self.plan.entries[index];
        plan_entry_height(entry, width, self.theme.glyphs)
    }

    fn revision(&self) -> u64 {
        self.plan.layout_revision()
    }
}

impl RowRenderer for PlanRows<'_> {
    fn render_row(&self, index: usize, area: Rect, buf: &mut Buffer, skip_top: u16) {
        let entry = &self.plan.entries[index];
        render_plan_entry(entry, self.theme, area, buf, skip_top);
    }
}

/// Total height of a plan rendered as one transcript cell.
#[must_use]
pub fn plan_cell_height(plan: &Plan, width: u16, glyphs: GlyphSet) -> u16 {
    if plan.entries.is_empty() {
        return 1;
    }
    plan.entries
        .iter()
        .map(|entry| plan_entry_height(entry, width, glyphs))
        .sum()
}

/// Render a plan transcript cell; `skip_top` skips leading rows when partially scrolled.
pub fn render_plan_cell(plan: &Plan, theme: &Theme, area: Rect, buf: &mut Buffer, skip_top: u16) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    if plan.entries.is_empty() {
        Paragraph::new(Line::from(Span::styled("(empty plan)", muted(theme)))).render(area, buf);
        return;
    }

    let mut skip = skip_top;
    let mut y = area.y;
    let mut remaining = area.height;

    for entry in &plan.entries {
        let entry_h = plan_entry_height(entry, area.width, theme.glyphs);
        if skip >= entry_h {
            skip -= entry_h;
            continue;
        }
        let entry_skip = skip;
        skip = 0;
        let draw_h = (entry_h - entry_skip).min(remaining);
        if draw_h == 0 {
            break;
        }
        let row_area = Rect {
            x: area.x,
            y,
            width: area.width,
            height: draw_h,
        };
        render_plan_entry(entry, theme, row_area, buf, entry_skip);
        y += draw_h;
        remaining -= draw_h;
        if remaining == 0 {
            break;
        }
    }
}

fn plan_entry_height(entry: &PlanEntry, width: u16, _glyphs: GlyphSet) -> u16 {
    let text_width = width.saturating_sub(MARKER_COLUMNS).max(1);
    let lines = wrap_lines(&entry.title, text_width);
    u16::try_from(lines.len().max(1)).unwrap_or(1)
}

fn render_plan_entry(
    entry: &PlanEntry,
    theme: &Theme,
    area: Rect,
    buf: &mut Buffer,
    skip_top: u16,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let marker = plan_marker(entry.status, theme.glyphs);
    let marker_style = plan_marker_style(entry.status, theme);
    let text_width = area.width.saturating_sub(MARKER_COLUMNS).max(1);
    let wrapped = wrap_lines(&entry.title, text_width);
    let title_style = plan_title_style(entry.status, theme);

    let mut lines: Vec<Line> = Vec::new();
    for (i, wline) in wrapped.iter().enumerate() {
        let mut spans = Vec::new();
        if i == 0 {
            spans.push(Span::styled(format!("{marker} "), marker_style));
        } else {
            spans.push(Span::styled("  ", fg(theme)));
        }
        spans.push(Span::styled(wline.text.clone(), title_style));
        lines.push(Line::from(spans));
    }
    if lines.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(format!("{marker} "), marker_style),
            Span::styled("", title_style),
        ]));
    }

    let line_count = u16::try_from(lines.len()).unwrap_or(u16::MAX);
    let start = usize::from(skip_top.min(line_count));
    let visible = lines.into_iter().skip(start).take(area.height as usize);
    Paragraph::new(visible.collect::<Vec<_>>()).render(area, buf);
}

fn plan_marker(status: PlanEntryStatus, glyphs: GlyphSet) -> &'static str {
    match (status, glyphs) {
        (PlanEntryStatus::Pending, GlyphSet::Ascii) => "[ ]",
        (PlanEntryStatus::InProgress, GlyphSet::Ascii) => "[>]",
        (PlanEntryStatus::Completed, GlyphSet::Ascii) => "[x]",
        (PlanEntryStatus::Pending, GlyphSet::Unicode) => "○",
        (PlanEntryStatus::InProgress, GlyphSet::Unicode) => "◉",
        (PlanEntryStatus::Completed, GlyphSet::Unicode) => "✓",
    }
}

fn plan_marker_style(status: PlanEntryStatus, theme: &Theme) -> ratatui_core::style::Style {
    match status {
        PlanEntryStatus::Pending => muted(theme),
        PlanEntryStatus::InProgress => accent(theme),
        PlanEntryStatus::Completed => success(theme),
    }
}

fn plan_title_style(status: PlanEntryStatus, theme: &Theme) -> ratatui_core::style::Style {
    match status {
        PlanEntryStatus::Pending => muted(theme),
        PlanEntryStatus::InProgress => fg(theme),
        PlanEntryStatus::Completed => success(theme),
    }
}
