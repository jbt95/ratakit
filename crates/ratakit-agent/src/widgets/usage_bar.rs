//! Token usage, context gauge, mode, and git branch on [`StatusBar`].

use ratakit_core::Theme;
use ratakit_widgets::{StatusBar, StatusSegment};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::Widget;

use crate::state::{ModeId, Usage};

/// Footer bar for agent session usage (built on generic [`StatusBar`] with priority truncation).
pub struct UsageBar<'a> {
    usage: &'a Usage,
    mode: &'a ModeId,
    git_branch: Option<&'a str>,
    theme: &'a Theme,
    compact: bool,
}

impl<'a> UsageBar<'a> {
    #[must_use]
    pub fn new(usage: &'a Usage, mode: &'a ModeId, theme: &'a Theme) -> Self {
        Self {
            usage,
            mode,
            git_branch: None,
            theme,
            compact: false,
        }
    }

    /// Fewer segments for narrow layouts (see [`usage_segments_compact`]).
    #[must_use]
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// Optional git branch shown on the right (lower priority than mode and context).
    #[must_use]
    pub fn git_branch(mut self, branch: Option<&'a str>) -> Self {
        self.git_branch = branch;
        self
    }
}

impl Widget for UsageBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let segments = if self.compact {
            usage_segments_compact(self.usage, self.mode)
        } else {
            usage_segments(self.usage, self.mode, self.git_branch)
        };
        StatusBar::new(self.theme, &segments).render(area, buf);
    }
}

/// Compact footer segments for narrow terminals (context gauge + mode only).
#[must_use]
pub fn usage_segments_compact(usage: &Usage, mode: &ModeId) -> Vec<StatusSegment> {
    let context = format_context_segment(usage.context_used);
    let mode_label = format!("mode {}", mode.status_label());
    vec![
        StatusSegment::center(context, 9),
        StatusSegment::right(mode_label, 10),
    ]
}

/// Build [`StatusSegment`]s for a custom layout or testing.
#[must_use]
pub fn usage_segments(
    usage: &Usage,
    mode: &ModeId,
    git_branch: Option<&str>,
) -> Vec<StatusSegment> {
    let tokens = format!(
        "in {} out {}",
        format_token_count(usage.input_tokens),
        format_token_count(usage.output_tokens)
    );
    let cached = format!("cached {}", format_token_count(usage.cached_tokens));
    let context = format_context_segment(usage.context_used);
    let mode_label = format!("mode {}", mode.status_label());
    let cost = format_cost(usage.cost_usd);
    let branch = git_branch.map(|b| format!("⎇ {b}")).unwrap_or_default();

    let mut segments = vec![
        StatusSegment::left(tokens, 8),
        StatusSegment::left(cached, 5),
        StatusSegment::center(context, 9),
        StatusSegment::right(mode_label, 10),
        StatusSegment::right(cost, 7),
    ];
    if !branch.is_empty() {
        segments.push(StatusSegment::right(branch, 6));
    }
    segments
}

fn format_token_count(n: u64) -> String {
    if n >= 1_000_000 {
        let whole = n / 1_000_000;
        let tenth = (n % 1_000_000) / 100_000;
        return format!("{whole}.{tenth}M");
    }
    if n >= 10_000 {
        return format!("{}k", n / 1_000);
    }
    if n >= 1_000 {
        let whole = n / 1_000;
        let tenth = (n % 1_000) / 100;
        return format!("{whole}.{tenth}k");
    }
    n.to_string()
}

fn format_cost(cost_usd: Option<f64>) -> String {
    match cost_usd {
        Some(c) if c >= 0.01 => format!("${c:.2}"),
        Some(c) if c > 0.0 => format!("${c:.4}"),
        Some(_) => "$0".to_string(),
        None => "cost —".to_string(),
    }
}

fn format_context_segment(context_used: Option<f32>) -> String {
    match context_used {
        Some(fraction) => {
            let pct = context_percent(fraction);
            let gauge = context_gauge_blocks(pct, 8);
            format!("{gauge} {pct}%")
        }
        None => "ctx —".to_string(),
    }
}

fn context_percent(fraction: f32) -> u32 {
    if fraction <= 0.0 {
        return 0;
    }
    if fraction >= 1.0 {
        return 100;
    }
    for p in 1..=100u32 {
        if fraction < f32::from(u16::try_from(p).unwrap_or(100)) / 100.0 {
            return p - 1;
        }
    }
    100
}

fn context_gauge_blocks(percent: u32, width: usize) -> String {
    let width = u32::try_from(width.clamp(1, 64)).unwrap_or(8);
    let percent = percent.min(100);
    let filled = (percent * width) / 100;
    let mut s = String::with_capacity(width as usize);
    for i in 0..width {
        s.push(if i < filled { '█' } else { '░' });
    }
    s
}
