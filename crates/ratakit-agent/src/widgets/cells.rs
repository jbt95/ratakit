//! Per-cell measure and paint for the transcript virtual list.

use crate::state::{
    AssistantCell, Cell, ErrorCell, Level, NoticeCell, Plan, ThinkingCell, ToolCallCell, UserCell,
};
use crate::widgets::extras::{
    PlaceholderPlanRenderer, PlaceholderToolCallRenderer, PlanCellRenderer, ToolCallCellRenderer,
    TranscriptExtras, label_line, render_wrapped_lines, thinking_summary, user_plain_text,
    wrapped_line_count,
};
use ratakit_core::Theme;
use ratakit_text::Highlighter;
use ratakit_widgets::semantic_to_color;
use ratakit_widgets::styled_line_to_ratatui_owned;
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Modifier, Style};
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;
use ratatui_widgets::paragraph::Paragraph;

const USER_LABEL: &str = "›";
const ASSISTANT_LABEL: &str = "◆";
const NOTICE_LABEL: &str = "·";
const ERROR_LABEL: &str = "!";

/// Populate markdown layout caches for the current frame width.
pub(crate) fn warm_markdown_layout(
    cells: &mut [Cell],
    width: u16,
    theme: &Theme,
    highlighter: &dyn Highlighter,
) {
    for cell in cells {
        match cell {
            Cell::Assistant(a) => {
                a.stream.lines(width, theme, highlighter);
            }
            Cell::Thinking(t) if !t.folded => {
                t.stream.lines(width, theme, highlighter);
            }
            _ => {}
        }
    }
}

pub(crate) fn measure_cell(
    cell: &Cell,
    width: u16,
    theme: &Theme,
    extras: TranscriptExtras<'_>,
) -> u16 {
    match cell {
        Cell::User(u) => measure_user(u, width, theme),
        Cell::Assistant(a) => measure_assistant(a, width, theme),
        Cell::Thinking(t) => measure_thinking(t, width, theme),
        Cell::ToolCall(tc) => measure_tool_call(tc, width, extras),
        Cell::Plan(p) => measure_plan(p, width, extras),
        Cell::Notice(n) => measure_notice(n, width),
        Cell::Error(e) => measure_error(e, width),
    }
}

pub(crate) fn render_cell(
    cell: &Cell,
    area: Rect,
    buf: &mut Buffer,
    skip_top: u16,
    theme: &Theme,
    extras: TranscriptExtras<'_>,
) {
    if area.height == 0 {
        return;
    }
    match cell {
        Cell::User(u) => render_user(u, area, buf, skip_top, theme),
        Cell::Assistant(a) => render_assistant(a, area, buf, skip_top, theme),
        Cell::Thinking(t) => render_thinking(t, area, buf, skip_top, theme),
        Cell::ToolCall(tc) => render_tool_call(tc, area, buf, skip_top, extras),
        Cell::Plan(p) => render_plan(p, area, buf, skip_top, extras),
        Cell::Notice(n) => render_notice(n, area, buf, skip_top, theme),
        Cell::Error(e) => render_error(e, area, buf, skip_top, theme),
    }
}

fn measure_user(user: &UserCell, width: u16, theme: &Theme) -> u16 {
    let body = user_plain_text(&user.content);
    let line = label_line(theme, USER_LABEL, &body);
    wrapped_line_count(&line, width)
}

fn render_user(user: &UserCell, area: Rect, buf: &mut Buffer, skip_top: u16, theme: &Theme) {
    let body = user_plain_text(&user.content);
    let line = label_line(theme, USER_LABEL, &body);
    render_wrapped_lines(&[&line], area, buf, skip_top, fg(theme.palette.accent));
}

fn measure_assistant(assistant: &AssistantCell, width: u16, theme: &Theme) -> u16 {
    let body_rows = assistant
        .stream
        .cached_line_count(width, theme)
        .unwrap_or_else(|| wrapped_line_count(assistant.stream.source(), width));
    body_rows + 1
}

fn render_assistant(
    assistant: &AssistantCell,
    area: Rect,
    buf: &mut Buffer,
    skip_top: u16,
    theme: &Theme,
) {
    let styled = assistant
        .stream
        .cached_lines(area.width, theme)
        .expect("markdown layout must be warmed before render");
    let mut rows: Vec<Line> = Vec::new();
    rows.push(Line::from(ASSISTANT_LABEL).style(fg(theme.palette.muted)));
    for line in styled {
        rows.push(styled_line_to_ratatui_owned(line));
    }
    render_line_stack(rows, area, buf, skip_top);
}

fn measure_thinking(thinking: &ThinkingCell, width: u16, theme: &Theme) -> u16 {
    if thinking.folded {
        return 1;
    }
    let body_rows = thinking
        .stream
        .cached_line_count(width, theme)
        .unwrap_or_else(|| wrapped_line_count(thinking.stream.source(), width));
    1 + body_rows
}

fn render_thinking(
    thinking: &ThinkingCell,
    area: Rect,
    buf: &mut Buffer,
    skip_top: u16,
    theme: &Theme,
) {
    let line_count = thinking
        .stream
        .cached_line_count(area.width, theme)
        .unwrap_or(1) as usize;
    let summary = thinking_summary(thinking.folded, thinking.done, line_count);
    if thinking.folded {
        render_wrapped_lines(&[&summary], area, buf, skip_top, fg(theme.palette.muted));
        return;
    }
    let styled = thinking
        .stream
        .cached_lines(area.width, theme)
        .expect("markdown layout must be warmed before render");
    let mut rows: Vec<Line> = Vec::new();
    rows.push(Line::from(summary).style(fg(theme.palette.muted).add_modifier(Modifier::ITALIC)));
    for line in styled {
        rows.push(styled_line_to_ratatui_owned(line));
    }
    render_line_stack(rows, area, buf, skip_top);
}

fn measure_tool_call(cell: &ToolCallCell, width: u16, extras: TranscriptExtras<'_>) -> u16 {
    if let Some(r) = extras.tool_call {
        r.height(cell, width)
    } else {
        PlaceholderToolCallRenderer.height(cell, width)
    }
}

fn render_tool_call(
    cell: &ToolCallCell,
    area: Rect,
    buf: &mut Buffer,
    skip_top: u16,
    extras: TranscriptExtras<'_>,
) {
    if let Some(r) = extras.tool_call {
        r.render(cell, area, buf, skip_top);
    } else {
        PlaceholderToolCallRenderer.render(cell, area, buf, skip_top);
    }
}

fn measure_plan(plan: &Plan, width: u16, extras: TranscriptExtras<'_>) -> u16 {
    if let Some(r) = extras.plan {
        r.height(plan, width)
    } else {
        PlaceholderPlanRenderer.height(plan, width)
    }
}

fn render_plan(
    plan: &Plan,
    area: Rect,
    buf: &mut Buffer,
    skip_top: u16,
    extras: TranscriptExtras<'_>,
) {
    if let Some(r) = extras.plan {
        r.render(plan, area, buf, skip_top);
    } else {
        PlaceholderPlanRenderer.render(plan, area, buf, skip_top);
    }
}

fn measure_notice(notice: &NoticeCell, width: u16) -> u16 {
    let line = format!("{NOTICE_LABEL} {}", notice.text);
    wrapped_line_count(&line, width)
}

fn render_notice(notice: &NoticeCell, area: Rect, buf: &mut Buffer, skip_top: u16, theme: &Theme) {
    let line = format!("{NOTICE_LABEL} {}", notice.text);
    let style = match notice.level {
        Level::Info => fg(theme.palette.muted),
        Level::Warning => fg(theme.palette.accent),
        Level::Error => fg(theme.palette.error),
    };
    render_wrapped_lines(&[&line], area, buf, skip_top, style);
}

fn measure_error(err: &ErrorCell, width: u16) -> u16 {
    let line = format!("{ERROR_LABEL} {}", err.text);
    wrapped_line_count(&line, width)
}

fn render_error(err: &ErrorCell, area: Rect, buf: &mut Buffer, skip_top: u16, theme: &Theme) {
    let line = format!("{ERROR_LABEL} {}", err.text);
    render_wrapped_lines(&[&line], area, buf, skip_top, fg(theme.palette.error));
}

fn fg(color: ratakit_core::SemanticColor) -> Style {
    Style::default().fg(semantic_to_color(color))
}

fn render_line_stack(lines: Vec<Line<'static>>, area: Rect, buf: &mut Buffer, skip_top: u16) {
    if area.height == 0 {
        return;
    }
    let start = skip_top.min(lines.len() as u16) as usize;
    let visible = lines.into_iter().skip(start).take(area.height as usize);
    Paragraph::new(visible.collect::<Vec<_>>()).render(area, buf);
}
