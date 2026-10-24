use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier, Style};
use std::fmt::Write as _;

/// Serialize a ratatui [`Buffer`] as plain text plus a style appendix (Codex-style).
pub fn buffer_to_string(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = buf[(x, y)].clone();
            let ch = if cell.symbol().is_empty() {
                ' '
            } else {
                cell.symbol().chars().next().unwrap_or(' ')
            };
            out.push(ch);
        }
        out.push('\n');
    }

    out.push_str("--- styles ---\n");
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = buf[(x, y)].clone();
            let style = cell.style();
            if is_effective_default(style) {
                continue;
            }
            let _ = writeln!(out, "({x},{y}): {}", style_to_string(style));
        }
    }
    out
}

fn is_effective_default(style: Style) -> bool {
    let no_mods = style.add_modifier.is_empty();
    let fg_default = style.fg.is_none() || style.fg == Some(Color::Reset);
    let bg_default = style.bg.is_none() || style.bg == Some(Color::Reset);
    no_mods && fg_default && bg_default
}

fn style_to_string(style: Style) -> String {
    let mut parts = Vec::new();
    if let Color::Rgb(r, g, b) = style.fg.unwrap_or(Color::Reset) {
        parts.push(format!("fg=rgb({r},{g},{b})"));
    } else if let Some(fg) = style.fg {
        parts.push(format!("fg={fg}"));
    }
    if let Color::Rgb(r, g, b) = style.bg.unwrap_or(Color::Reset) {
        parts.push(format!("bg=rgb({r},{g},{b})"));
    } else if let Some(bg) = style.bg {
        parts.push(format!("bg={bg}"));
    }
    let mods = style.add_modifier;
    if mods.contains(Modifier::BOLD) {
        parts.push("bold".into());
    }
    if mods.contains(Modifier::ITALIC) {
        parts.push("italic".into());
    }
    if mods.contains(Modifier::UNDERLINED) {
        parts.push("underline".into());
    }
    if parts.is_empty() {
        "default".into()
    } else {
        parts.join(" ")
    }
}
