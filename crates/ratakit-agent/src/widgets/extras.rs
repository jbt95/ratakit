//! Shared text helpers for transcript cells.

use ratatui_core::style::Style;
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;
use ratatui_widgets::paragraph::Paragraph;
use unicode_width::UnicodeWidthStr;

use ratakit_core::Theme;

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
    area: ratatui_core::layout::Rect,
    buf: &mut ratatui_core::buffer::Buffer,
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
