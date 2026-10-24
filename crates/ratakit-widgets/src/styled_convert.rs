//! Convert ratakit-text [`StyledLine`] into ratatui types.

use ratakit_core::{Rgb, SemanticColor};
use ratakit_text::{StyledLine, StyledSpan, TextModifier, TextStyle};
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::text::{Line, Span};

/// Map a semantic color to ratatui [`Color`].
pub fn semantic_to_color(color: SemanticColor) -> Color {
    match color {
        SemanticColor::Rgb(Rgb { r, g, b }) => Color::Rgb(r, g, b),
        SemanticColor::Index(i) => Color::Indexed(i),
    }
}

/// Convert [`TextStyle`] to ratatui [`Style`].
pub fn text_style_to_ratatui(style: TextStyle) -> Style {
    let mut out = Style::new();
    if let Some(fg) = style.fg {
        out = out.fg(semantic_to_color(fg));
    }
    if let Some(bg) = style.bg {
        out = out.bg(semantic_to_color(bg));
    }
    let mut mods = Modifier::empty();
    if style.modifier.contains(TextModifier::BOLD) {
        mods |= Modifier::BOLD;
    }
    if style.modifier.contains(TextModifier::ITALIC) {
        mods |= Modifier::ITALIC;
    }
    if style.modifier.contains(TextModifier::UNDERLINE) {
        mods |= Modifier::UNDERLINED;
    }
    if style.modifier.contains(TextModifier::DIM) {
        mods |= Modifier::DIM;
    }
    out.add_modifier(mods)
}

/// Convert one [`StyledSpan`].
pub fn styled_span_to_ratatui(span: &StyledSpan) -> Span<'_> {
    Span::styled(span.text.as_str(), text_style_to_ratatui(span.style))
}

/// Convert a [`StyledLine`] to ratatui [`Line`] (borrows span text).
pub fn styled_line_to_ratatui(line: &StyledLine) -> Line<'_> {
    let spans: Vec<Span<'_>> = line.spans.iter().map(styled_span_to_ratatui).collect();
    Line::from(spans)
}

/// Convert a [`StyledLine`] with owned span text (for caching rendered rows).
pub fn styled_line_to_ratatui_owned(line: &StyledLine) -> Line<'static> {
    let spans: Vec<Span<'static>> = line
        .spans
        .iter()
        .map(|s| Span::styled(s.text.clone(), text_style_to_ratatui(s.style)))
        .collect();
    Line::from(spans)
}

/// Convert many lines for ratatui's `Paragraph` widget.
pub fn styled_lines_to_ratatui(lines: &[StyledLine]) -> Vec<Line<'_>> {
    lines.iter().map(styled_line_to_ratatui).collect()
}
