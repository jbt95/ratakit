use ratakit_core::{GlyphSet, Theme};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Modifier, Style};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;
use ratatui_widgets::block::Block;
use ratatui_widgets::borders::BorderType;
use ratatui_widgets::paragraph::Paragraph;

use crate::style::{accent, border, error, fg, muted, palette_bg, semantic_color};

/// Visual treatment for a [`Button`] or [`Badge`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Variant {
    #[default]
    Default,
    Secondary,
    Destructive,
    Outline,
    Ghost,
    Link,
}

/// Horizontal density for a [`Button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    Small,
    #[default]
    Default,
    Large,
}

/// A single-line action label. The application owns activation and hit testing.
pub struct Button<'a> {
    theme: &'a Theme,
    label: &'a str,
    variant: Variant,
    size: ButtonSize,
    focused: bool,
    disabled: bool,
}

impl<'a> Button<'a> {
    pub fn new(theme: &'a Theme, label: &'a str) -> Self {
        Self {
            theme,
            label,
            variant: Variant::Default,
            size: ButtonSize::Default,
            focused: false,
            disabled: false,
        }
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn style(&self) -> Style {
        if self.disabled {
            return muted(self.theme).add_modifier(Modifier::DIM);
        }
        if self.focused {
            return Style::default()
                .fg(semantic_color(self.theme.palette.background))
                .bg(semantic_color(self.theme.palette.accent))
                .add_modifier(Modifier::BOLD);
        }
        match self.variant {
            Variant::Default => Style::default()
                .fg(semantic_color(self.theme.palette.background))
                .bg(semantic_color(self.theme.palette.accent))
                .add_modifier(Modifier::BOLD),
            Variant::Secondary => fg(self.theme)
                .bg(semantic_color(self.theme.palette.border))
                .add_modifier(Modifier::BOLD),
            Variant::Destructive => error(self.theme).add_modifier(Modifier::BOLD),
            Variant::Outline => border(self.theme),
            Variant::Ghost => fg(self.theme),
            Variant::Link => accent(self.theme).add_modifier(Modifier::UNDERLINED),
        }
    }
}

impl Widget for Button<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let padding = match self.size {
            ButtonSize::Small => 0,
            ButtonSize::Default => 1,
            ButtonSize::Large => 2,
        };
        let label = if self.disabled && self.theme.screen_reader {
            format!("{} (disabled)", self.label)
        } else {
            self.label.to_owned()
        };
        let line = if self.variant == Variant::Outline && !self.disabled {
            let padding = " ".repeat(padding);
            Line::from(vec![
                Span::styled("[", border(self.theme)),
                Span::styled(padding.clone(), fg(self.theme)),
                Span::styled(label, self.style()),
                Span::styled(padding, fg(self.theme)),
                Span::styled("]", border(self.theme)),
            ])
        } else {
            Line::from(Span::styled(
                format!("{}{}{}", " ".repeat(padding), label, " ".repeat(padding)),
                self.style(),
            ))
        };
        Paragraph::new(line).render(area, buf);
    }
}

/// Compact inline label using the same semantic variants as [`Button`].
pub struct Badge<'a> {
    theme: &'a Theme,
    label: &'a str,
    variant: Variant,
}

impl<'a> Badge<'a> {
    pub fn new(theme: &'a Theme, label: &'a str) -> Self {
        Self {
            theme,
            label,
            variant: Variant::Default,
        }
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }
}

impl Widget for Badge<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let style = match self.variant {
            Variant::Default => Style::default()
                .fg(semantic_color(self.theme.palette.background))
                .bg(semantic_color(self.theme.palette.accent))
                .add_modifier(Modifier::BOLD),
            Variant::Secondary => fg(self.theme)
                .bg(semantic_color(self.theme.palette.border))
                .add_modifier(Modifier::BOLD),
            Variant::Destructive => error(self.theme).add_modifier(Modifier::BOLD),
            Variant::Outline => border(self.theme),
            Variant::Ghost => fg(self.theme),
            Variant::Link => accent(self.theme).add_modifier(Modifier::UNDERLINED),
        };
        let line = if self.variant == Variant::Outline {
            Line::from(vec![
                Span::styled("[", border(self.theme)),
                Span::styled(self.label, fg(self.theme)),
                Span::styled("]", border(self.theme)),
            ])
        } else {
            Line::from(Span::styled(format!(" {} ", self.label), style))
        };
        Paragraph::new(line).render(area, buf);
    }
}

/// Rounded bordered surface for composing child widgets.
///
/// Render the card first, then render children into [`inner`](Self::inner).
pub struct Card<'a> {
    theme: &'a Theme,
    title: Option<&'a str>,
}

impl<'a> Card<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self { theme, title: None }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn inner(&self, area: Rect) -> Rect {
        self.block().inner(area)
    }

    fn block(&self) -> Block<'a> {
        let border_type = if self.theme.glyphs == GlyphSet::Ascii {
            BorderType::Plain
        } else {
            BorderType::Rounded
        };
        let block = Block::bordered()
            .border_type(border_type)
            .style(palette_bg(self.theme).patch(border(self.theme)));
        match self.title {
            Some(title) => block.title(Line::from(title).style(fg(self.theme))),
            None => block,
        }
    }
}

impl Widget for Card<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let block = self.block();
        block.render(area, buf);
    }
}

/// Direction in which a [`Separator`] spans its rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Theme-aware one-cell divider.
pub struct Separator<'a> {
    theme: &'a Theme,
    orientation: Orientation,
}

impl<'a> Separator<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            theme,
            orientation: Orientation::Horizontal,
        }
    }

    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }
}

impl Widget for Separator<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let symbol = match (self.theme.glyphs, self.orientation) {
            (GlyphSet::Ascii, Orientation::Horizontal) => "-",
            (GlyphSet::Ascii, Orientation::Vertical) => "|",
            (GlyphSet::Unicode, Orientation::Horizontal) => "─",
            (GlyphSet::Unicode, Orientation::Vertical) => "│",
        };
        let style = border(self.theme);
        match self.orientation {
            Orientation::Horizontal => {
                let y = area.y + area.height / 2;
                for x in area.left()..area.right() {
                    let cell = &mut buf[(x, y)];
                    cell.set_symbol(symbol);
                    cell.set_style(style);
                }
            }
            Orientation::Vertical => {
                let x = area.x + area.width / 2;
                for y in area.top()..area.bottom() {
                    let cell = &mut buf[(x, y)];
                    cell.set_symbol(symbol);
                    cell.set_style(style);
                }
            }
        }
    }
}

/// Determinate single-line progress bar.
pub struct Progress<'a> {
    theme: &'a Theme,
    value: f64,
    show_percentage: bool,
}

impl<'a> Progress<'a> {
    pub fn new(theme: &'a Theme, value: f64) -> Self {
        Self {
            theme,
            value: if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                0.0
            },
            show_percentage: true,
        }
    }

    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }
}

impl Widget for Progress<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let percentage = (self.value * 100.0).round() as u8;
        let percentage_text = format!("{percentage:>3}%");
        let show_percentage = self.show_percentage && area.width >= 7;
        let bar_width = area
            .width
            .saturating_sub(if show_percentage { 4 } else { 0 });
        let filled = (self.value * f64::from(bar_width)).round() as u16;
        let (full, empty) = if self.theme.glyphs == GlyphSet::Ascii {
            ("#", "-")
        } else {
            ("█", "░")
        };
        for offset in 0..bar_width {
            let cell = &mut buf[(area.x + offset, area.y)];
            cell.set_symbol(if offset < filled { full } else { empty });
            cell.set_style(if offset < filled {
                accent(self.theme)
            } else {
                muted(self.theme)
            });
        }
        if show_percentage {
            Paragraph::new(percentage_text)
                .style(fg(self.theme))
                .render(
                    Rect::new(area.x + bar_width, area.y, area.width - bar_width, 1),
                    buf,
                );
        }
    }
}

/// Static placeholder bars for loading layouts; no timer or animation is created.
pub struct Skeleton<'a> {
    theme: &'a Theme,
}

impl<'a> Skeleton<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }
}

impl Widget for Skeleton<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let symbol = if self.theme.glyphs == GlyphSet::Ascii {
            "-"
        } else {
            "░"
        };
        let style = muted(self.theme).add_modifier(Modifier::DIM);
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = &mut buf[(x, y)];
                cell.set_symbol(symbol);
                cell.set_style(style);
            }
        }
    }
}
