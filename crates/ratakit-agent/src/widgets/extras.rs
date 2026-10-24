//! Pluggable renderers for transcript cells built in parallel milestones (A3, A7).

use crate::state::{Plan, ToolCallCell};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;
use ratatui_widgets::paragraph::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::state::ToolStatus;
use ratakit_core::Theme;

/// Renders a [`ToolCallCell`] inside the transcript (see milestone A3).
pub trait ToolCallCellRenderer {
    /// Height in terminal rows at `width`.
    fn height(&self, cell: &ToolCallCell, width: u16) -> u16;

    /// Draw the cell, skipping the first `skip_top` rows of content.
    fn render(&self, cell: &ToolCallCell, area: Rect, buf: &mut Buffer, skip_top: u16);
}

/// Renders a [`Plan`] transcript cell (see milestone A7).
pub trait PlanCellRenderer {
    fn height(&self, plan: &Plan, width: u16) -> u16;
    fn render(&self, plan: &Plan, area: Rect, buf: &mut Buffer, skip_top: u16);
}

/// Optional overrides for tool-call and plan rows.
#[derive(Clone, Copy, Default)]
pub struct TranscriptExtras<'a> {
    pub tool_call: Option<&'a dyn ToolCallCellRenderer>,
    pub plan: Option<&'a dyn PlanCellRenderer>,
}

/// Simple placeholder until `ToolCallCard` lands (A3).
#[derive(Debug, Clone, Copy, Default)]
pub struct PlaceholderToolCallRenderer;

impl ToolCallCellRenderer for PlaceholderToolCallRenderer {
    fn height(&self, cell: &ToolCallCell, width: u16) -> u16 {
        wrapped_line_count(&placeholder_tool_line(cell), width)
    }

    fn render(&self, cell: &ToolCallCell, area: Rect, buf: &mut Buffer, skip_top: u16) {
        let line = placeholder_tool_line(cell);
        render_wrapped_lines(&[line.as_str()], area, buf, skip_top, Style::default());
    }
}

/// Simple placeholder until `PlanView` lands (A7).
#[derive(Debug, Clone, Copy, Default)]
pub struct PlaceholderPlanRenderer;

impl PlanCellRenderer for PlaceholderPlanRenderer {
    fn height(&self, plan: &Plan, width: u16) -> u16 {
        if plan.entries.is_empty() {
            return 1;
        }
        let lines: Vec<String> = plan
            .entries
            .iter()
            .map(|e| format!("- [{}] {}", plan_status_char(e.status), e.title))
            .collect();
        lines
            .iter()
            .map(|l| wrapped_line_count(l, width))
            .sum::<u16>()
            .max(1)
    }

    fn render(&self, plan: &Plan, area: Rect, buf: &mut Buffer, skip_top: u16) {
        let lines: Vec<String> = if plan.entries.is_empty() {
            vec!["(empty plan)".into()]
        } else {
            plan.entries
                .iter()
                .map(|e| format!("- [{}] {}", plan_status_char(e.status), e.title))
                .collect()
        };
        render_wrapped_lines(
            &lines.iter().map(String::as_str).collect::<Vec<_>>(),
            area,
            buf,
            skip_top,
            Style::default(),
        );
    }
}

fn placeholder_tool_line(cell: &ToolCallCell) -> String {
    let title = if cell.call.title.is_empty() {
        "(tool)"
    } else {
        cell.call.title.as_str()
    };
    format!(
        "[tool {title} · {status}]",
        status = tool_status_label(cell.call.status)
    )
}

fn tool_status_label(status: ToolStatus) -> &'static str {
    match status {
        ToolStatus::Pending => "pending",
        ToolStatus::InProgress => "running",
        ToolStatus::Completed => "done",
        ToolStatus::Failed => "failed",
        ToolStatus::Cancelled => "cancelled",
    }
}

fn plan_status_char(status: crate::state::PlanEntryStatus) -> char {
    use crate::state::PlanEntryStatus;
    match status {
        PlanEntryStatus::Pending => ' ',
        PlanEntryStatus::InProgress => '>',
        PlanEntryStatus::Completed => 'x',
    }
}

pub(crate) fn wrapped_line_count(text: &str, width: u16) -> u16 {
    if text.is_empty() {
        return 1;
    }
    let cols = width.max(1) as usize;
    let rows = text
        .lines()
        .map(|line| line.width().div_ceil(cols).max(1))
        .sum::<usize>()
        .max(1);
    u16::try_from(rows).unwrap_or(u16::MAX)
}

pub(crate) fn render_wrapped_lines(
    lines: &[&str],
    area: Rect,
    buf: &mut Buffer,
    skip_top: u16,
    style: Style,
) {
    if area.height == 0 {
        return;
    }
    let cols = area.width.max(1) as usize;
    let mut wrapped: Vec<Line> = Vec::new();
    for line in lines {
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            wrapped.push(Line::from(""));
            continue;
        }
        for chunk in chars.chunks(cols) {
            wrapped.push(Line::from(chunk.iter().collect::<String>()));
        }
    }
    if wrapped.is_empty() {
        wrapped.push(Line::from(""));
    }
    let line_cap = u16::try_from(wrapped.len()).unwrap_or(u16::MAX);
    let start = skip_top.min(line_cap) as usize;
    let visible = wrapped.into_iter().skip(start).take(area.height as usize);
    Paragraph::new(visible.collect::<Vec<_>>())
        .style(style)
        .render(area, buf);
}

pub(crate) fn thinking_summary(folded: bool, done: bool, line_count: usize) -> String {
    let marker = if folded { '▸' } else { '▾' };
    let status = if done { "done" } else { "streaming" };
    format!("{marker} Thinking ({status}, {line_count} lines)")
}

pub(crate) fn user_plain_text(parts: &[crate::state::ContentPart]) -> String {
    let mut out = String::new();
    for part in parts {
        match part {
            crate::state::ContentPart::Text(t) => {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(t);
            }
            crate::state::ContentPart::Image { alt } => {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str("[image: ");
                out.push_str(alt);
                out.push(']');
            }
        }
    }
    out
}

pub(crate) fn label_line(_theme: &Theme, label: &str, body: &str) -> String {
    format!("{label} {body}")
}
