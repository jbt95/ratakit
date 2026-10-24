use ratakit_core::ToastLevel;
use ratakit_core::{SemanticColor, Theme};
use ratatui_core::style::{Color, Modifier, Style};

/// Convert a semantic palette color to a ratatui [`Color`].
pub fn semantic_color(color: SemanticColor) -> Color {
    match color {
        SemanticColor::Rgb(rgb) => Color::Rgb(rgb.r, rgb.g, rgb.b),
        SemanticColor::Index(i) => Color::Indexed(i),
    }
}

pub fn fg_style(theme: &Theme) -> Style {
    Style::new().fg(semantic_color(theme.palette.foreground))
}

pub fn muted_style(theme: &Theme) -> Style {
    Style::new().fg(semantic_color(theme.palette.muted))
}

pub fn accent_style(theme: &Theme) -> Style {
    Style::new().fg(semantic_color(theme.palette.accent))
}

pub fn status_bar_style(theme: &Theme) -> Style {
    Style::new()
        .fg(semantic_color(theme.palette.foreground))
        .bg(semantic_color(theme.palette.background))
}

pub fn toast_style(theme: &Theme, level: ToastLevel) -> Style {
    let (fg, bg) = match level {
        ToastLevel::Info => (theme.palette.foreground, theme.palette.background),
        ToastLevel::Success => (theme.palette.success, theme.palette.background),
        ToastLevel::Warning => (theme.palette.warning, theme.palette.background),
        ToastLevel::Error => (theme.palette.error, theme.palette.background),
    };
    Style::new()
        .fg(semantic_color(fg))
        .bg(semantic_color(bg))
        .add_modifier(Modifier::BOLD)
}

#[cfg(test)]
mod tests {
    use ratakit_core::{Theme, ToastLevel};

    use super::{semantic_color, toast_style};

    #[test]
    fn info_toast_uses_the_palette_background_for_text_contrast() {
        for theme in [Theme::high_contrast_dark(), Theme::high_contrast_light()] {
            let style = toast_style(&theme, ToastLevel::Info);
            assert_eq!(style.fg, Some(semantic_color(theme.palette.foreground)));
            assert_eq!(style.bg, Some(semantic_color(theme.palette.background)));
        }
    }
}
