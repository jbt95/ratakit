use ratakit_core::{SemanticColor, Theme};
use ratatui_core::style::{Color, Modifier, Style};

/// Converts a semantic palette color to a ratatui [`Color`].
#[must_use]
pub fn semantic_color(color: SemanticColor) -> Color {
    match color {
        SemanticColor::Rgb(rgb) => Color::Rgb(rgb.r, rgb.g, rgb.b),
        SemanticColor::Index(index) => Color::Indexed(index),
    }
}

/// Default foreground style from the theme palette.
#[must_use]
pub fn fg_style(theme: &Theme) -> Style {
    Style::default().fg(semantic_color(theme.palette.foreground))
}

/// Muted foreground (secondary text).
#[must_use]
pub fn muted_style(theme: &Theme) -> Style {
    Style::default().fg(semantic_color(theme.palette.muted))
}

/// Style for a selected list row.
#[must_use]
pub fn selected_row_style(theme: &Theme) -> Style {
    fg_style(theme).add_modifier(Modifier::REVERSED)
}
