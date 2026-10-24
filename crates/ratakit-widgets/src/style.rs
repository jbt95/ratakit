use ratakit_core::{SemanticColor, Theme};
use ratatui_core::style::{Color, Modifier, Style};

pub fn semantic_color(color: SemanticColor) -> Color {
    match color {
        SemanticColor::Rgb(rgb) => Color::Rgb(rgb.r, rgb.g, rgb.b),
        SemanticColor::Index(i) => Color::Indexed(i),
    }
}

pub fn fg(theme: &Theme) -> Style {
    Style::default().fg(semantic_color(theme.palette.foreground))
}

pub fn muted(theme: &Theme) -> Style {
    Style::default().fg(semantic_color(theme.palette.muted))
}

pub fn accent(theme: &Theme) -> Style {
    Style::default().fg(semantic_color(theme.palette.accent))
}

pub fn border(theme: &Theme) -> Style {
    Style::default().fg(semantic_color(theme.palette.border))
}

pub fn cursor_style(theme: &Theme) -> Style {
    Style::default()
        .fg(semantic_color(theme.palette.background))
        .bg(semantic_color(theme.palette.accent))
        .add_modifier(Modifier::BOLD)
}

pub fn palette_bg(theme: &Theme) -> Style {
    Style::default().bg(semantic_color(theme.palette.background))
}

pub fn error(theme: &Theme) -> Style {
    Style::default().fg(semantic_color(theme.palette.error))
}

/// Style for a selected virtual list row.
pub fn selected_row_style(theme: &Theme) -> Style {
    fg(theme).add_modifier(Modifier::REVERSED)
}
