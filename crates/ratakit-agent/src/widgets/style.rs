use ratakit_core::{SemanticColor, Theme};
use ratatui_core::style::{Color, Style};

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

pub fn success(theme: &Theme) -> Style {
    Style::default().fg(semantic_color(theme.palette.success))
}
