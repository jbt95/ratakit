use ratakit_core::Theme;
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;
use ratatui_widgets::paragraph::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::theme_util::status_bar_style;

/// Horizontal zone in a [`StatusBar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusZone {
    Left,
    Center,
    Right,
}

/// One piece of status text; higher `priority` is kept when space is tight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSegment {
    pub text: String,
    pub priority: u8,
    pub zone: StatusZone,
}

impl StatusSegment {
    pub fn left(text: impl Into<String>, priority: u8) -> Self {
        Self {
            text: text.into(),
            zone: StatusZone::Left,
            priority,
        }
    }

    pub fn center(text: impl Into<String>, priority: u8) -> Self {
        Self {
            text: text.into(),
            zone: StatusZone::Center,
            priority,
        }
    }

    pub fn right(text: impl Into<String>, priority: u8) -> Self {
        Self {
            text: text.into(),
            zone: StatusZone::Right,
            priority,
        }
    }
}

/// Footer-style bar with left, center, and right segments and priority truncation.
pub struct StatusBar<'a> {
    theme: &'a Theme,
    segments: &'a [StatusSegment],
}

impl<'a> StatusBar<'a> {
    pub fn new(theme: &'a Theme, segments: &'a [StatusSegment]) -> Self {
        Self { theme, segments }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let width = area.width as usize;
        let (left, center, right) = layout_segments(self.segments, width);
        let line = compose_line(&left, &center, &right, width);
        let style = status_bar_style(self.theme);
        Paragraph::new(Line::from(Span::styled(line, style))).render(area, buf);
    }
}

fn layout_segments(segments: &[StatusSegment], width: usize) -> (String, String, String) {
    let mut left: Vec<&StatusSegment> = segments
        .iter()
        .filter(|s| s.zone == StatusZone::Left)
        .collect();
    let mut center: Vec<&StatusSegment> = segments
        .iter()
        .filter(|s| s.zone == StatusZone::Center)
        .collect();
    let mut right: Vec<&StatusSegment> = segments
        .iter()
        .filter(|s| s.zone == StatusZone::Right)
        .collect();

    for group in [&mut left, &mut center, &mut right] {
        group.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    let join = |parts: &[&StatusSegment]| -> String {
        parts
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" · ")
    };

    let mut left_s = join(&left);
    let mut center_s = join(&center);
    let mut right_s = join(&right);

    truncate_row(&mut left_s, &mut center_s, &mut right_s, width);
    (left_s, center_s, right_s)
}

fn truncate_row(left: &mut String, center: &mut String, right: &mut String, width: usize) {
    const ELLIPSIS: &str = "…";

    let total = left.width() + center.width() + right.width();
    if total <= width {
        return;
    }

    // Drop lowest-priority segments by shortening strings from the edges inward.
    let shrink = |s: &mut String, max: usize| {
        if s.width() <= max {
            return;
        }
        if max <= ELLIPSIS.width() {
            s.clear();
            return;
        }
        let target = max - ELLIPSIS.width();
        let mut out = String::new();
        let mut w = 0;
        for ch in s.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if w + cw > target {
                break;
            }
            out.push(ch);
            w += cw;
        }
        out.push('…');
        *s = out;
    };

    // Allocate: right and left get equal share; center gets the middle remainder.
    let third = width / 3;
    shrink(right, third);
    shrink(left, third);
    let used = left.width() + right.width();
    let center_budget = width.saturating_sub(used);
    shrink(center, center_budget);
}

fn compose_line(left: &str, center: &str, right: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let left_w = left.width();
    let center_w = center.width();
    let right_w = right.width();
    let total = left_w + center_w + right_w;

    if total <= width {
        if center.is_empty() {
            let pad = width - left_w - right_w;
            return format!("{left}{pad}{right}", pad = " ".repeat(pad));
        }
        let pad_left = (width - total) / 2;
        let mut out = String::new();
        out.push_str(left);
        for _ in 0..pad_left {
            out.push(' ');
        }
        out.push_str(center);
        while out.width() < width - right_w {
            out.push(' ');
        }
        out.push_str(right);
        while out.width() < width {
            out.push(' ');
        }
        return out;
    }

    // Hard truncate combined string
    let combined = format!("{left}{center}{right}");
    truncate_to_width(&combined, width)
}

fn truncate_to_width(s: &str, max: usize) -> String {
    if s.width() <= max {
        return s.to_string();
    }
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > max.saturating_sub(1) {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('…');
    out
}
