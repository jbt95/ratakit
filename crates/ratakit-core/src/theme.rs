use crate::Revisioned;

/// Semantic colors independent of ratatui style types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticColor {
    Rgb(Rgb),
    Index(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    pub background: SemanticColor,
    pub foreground: SemanticColor,
    pub accent: SemanticColor,
    pub success: SemanticColor,
    pub warning: SemanticColor,
    pub error: SemanticColor,
    pub muted: SemanticColor,
    pub border: SemanticColor,
}

/// Semantic text color included in [`Palette::text_contrast_ratios`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteTextRole {
    Foreground,
    Accent,
    Success,
    Warning,
    Error,
    Muted,
}

/// Measured contrast for one semantic text color against the palette background.
///
/// `ratio` is `None` when either color uses a terminal palette index, whose RGB value is
/// controlled by the terminal and cannot be measured portably.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteTextContrast {
    pub role: PaletteTextRole,
    pub ratio: Option<f64>,
}

impl Palette {
    /// Measure all semantic text colors against this palette's background.
    pub fn text_contrast_ratios(&self) -> [PaletteTextContrast; 6] {
        [
            (PaletteTextRole::Foreground, self.foreground),
            (PaletteTextRole::Accent, self.accent),
            (PaletteTextRole::Success, self.success),
            (PaletteTextRole::Warning, self.warning),
            (PaletteTextRole::Error, self.error),
            (PaletteTextRole::Muted, self.muted),
        ]
        .map(|(role, color)| PaletteTextContrast {
            role,
            ratio: contrast_ratio(color, self.background),
        })
    }

    pub fn dark_defaults() -> Self {
        Self {
            background: SemanticColor::Rgb(Rgb {
                r: 0x1e,
                g: 0x1e,
                b: 0x2e,
            }),
            foreground: SemanticColor::Rgb(Rgb {
                r: 0xcd,
                g: 0xd6,
                b: 0xf4,
            }),
            accent: SemanticColor::Rgb(Rgb {
                r: 0x89,
                g: 0xb4,
                b: 0xfa,
            }),
            success: SemanticColor::Rgb(Rgb {
                r: 0xa6,
                g: 0xe3,
                b: 0xa1,
            }),
            warning: SemanticColor::Rgb(Rgb {
                r: 0xf9,
                g: 0xe2,
                b: 0xaf,
            }),
            error: SemanticColor::Rgb(Rgb {
                r: 0xf3,
                g: 0x8b,
                b: 0xa8,
            }),
            muted: SemanticColor::Rgb(Rgb {
                r: 0x6c,
                g: 0x70,
                b: 0x86,
            }),
            border: SemanticColor::Rgb(Rgb {
                r: 0x45,
                g: 0x47,
                b: 0x5a,
            }),
        }
    }

    pub fn light_defaults() -> Self {
        Self {
            background: SemanticColor::Rgb(Rgb {
                r: 0xef,
                g: 0xf1,
                b: 0xf5,
            }),
            foreground: SemanticColor::Rgb(Rgb {
                r: 0x1e,
                g: 0x1e,
                b: 0x2e,
            }),
            accent: SemanticColor::Rgb(Rgb {
                r: 0x1e,
                g: 0x66,
                b: 0xf5,
            }),
            success: SemanticColor::Rgb(Rgb {
                r: 0x40,
                g: 0xa0,
                b: 0x2b,
            }),
            warning: SemanticColor::Rgb(Rgb {
                r: 0xdf,
                g: 0x8e,
                b: 0x1d,
            }),
            error: SemanticColor::Rgb(Rgb {
                r: 0xd2,
                g: 0x0f,
                b: 0x39,
            }),
            muted: SemanticColor::Rgb(Rgb {
                r: 0x6c,
                g: 0x6f,
                b: 0x85,
            }),
            border: SemanticColor::Rgb(Rgb {
                r: 0xac,
                g: 0xb0,
                b: 0xbe,
            }),
        }
    }

    /// Black background and bright semantic colors for high-contrast dark terminals.
    pub fn high_contrast_dark() -> Self {
        Self {
            background: rgb(0x00, 0x00, 0x00),
            foreground: rgb(0xff, 0xff, 0xff),
            accent: rgb(0x00, 0xe5, 0xff),
            success: rgb(0x00, 0xff, 0x5a),
            warning: rgb(0xff, 0xff, 0x00),
            error: rgb(0xff, 0x85, 0x85),
            muted: rgb(0xd6, 0xd6, 0xd6),
            border: rgb(0xa0, 0xa0, 0xa0),
        }
    }

    /// White background and dark semantic colors for high-contrast light terminals.
    pub fn high_contrast_light() -> Self {
        Self {
            background: rgb(0xff, 0xff, 0xff),
            foreground: rgb(0x00, 0x00, 0x00),
            accent: rgb(0x00, 0x33, 0xcc),
            success: rgb(0x00, 0x5a, 0x24),
            warning: rgb(0x70, 0x45, 0x00),
            error: rgb(0xa0, 0x00, 0x00),
            muted: rgb(0x40, 0x40, 0x40),
            border: rgb(0x59, 0x59, 0x59),
        }
    }
}

/// Calculate WCAG relative-luminance contrast for two explicit RGB colors.
///
/// Returns `None` for indexed colors because the actual RGB values depend on the user's
/// terminal palette.
pub fn contrast_ratio(foreground: SemanticColor, background: SemanticColor) -> Option<f64> {
    let foreground = color_luminance(foreground)?;
    let background = color_luminance(background)?;
    Some((foreground.max(background) + 0.05) / (foreground.min(background) + 0.05))
}

fn color_luminance(color: SemanticColor) -> Option<f64> {
    let SemanticColor::Rgb(rgb) = color else {
        return None;
    };
    Some(luminance(rgb))
}

fn luminance(rgb: Rgb) -> f64 {
    fn linear(channel: u8) -> f64 {
        let channel = f64::from(channel) / 255.0;
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126 * linear(rgb.r) + 0.7152 * linear(rgb.g) + 0.0722 * linear(rgb.b)
}

const fn rgb(r: u8, g: u8, b: u8) -> SemanticColor {
    SemanticColor::Rgb(Rgb { r, g, b })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MotionMode {
    #[default]
    Full,
    Reduced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GlyphSet {
    #[default]
    Unicode,
    Ascii,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxThemeId(pub &'static str);

impl SyntaxThemeId {
    pub const BASE16_MARDUK: Self = Self("base16-marduk.dark");
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub palette: Palette,
    pub syntax: SyntaxThemeId,
    pub motion: MotionMode,
    pub glyphs: GlyphSet,
    /// Text-first labels and static output intended for screen-reader workflows.
    pub screen_reader: bool,
    revision: u64,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            palette: Palette::dark_defaults(),
            syntax: SyntaxThemeId::BASE16_MARDUK,
            motion: MotionMode::Full,
            glyphs: GlyphSet::Unicode,
            screen_reader: false,
            revision: 0,
        }
    }

    pub fn light() -> Self {
        Self {
            palette: Palette::light_defaults(),
            syntax: SyntaxThemeId::BASE16_MARDUK,
            motion: MotionMode::Full,
            glyphs: GlyphSet::Unicode,
            screen_reader: false,
            revision: 0,
        }
    }

    /// High-contrast theme for dark terminal backgrounds.
    pub fn high_contrast_dark() -> Self {
        Self {
            palette: Palette::high_contrast_dark(),
            ..Self::dark()
        }
    }

    /// High-contrast theme for light terminal backgrounds.
    pub fn high_contrast_light() -> Self {
        Self {
            palette: Palette::high_contrast_light(),
            ..Self::light()
        }
    }

    /// Enable text-first labels, ASCII markers, and reduced motion.
    ///
    /// The selected motion and glyph settings can still be customized separately.
    pub fn enable_screen_reader(&mut self) {
        if !self.screen_reader
            || self.motion != MotionMode::Reduced
            || self.glyphs != GlyphSet::Ascii
        {
            self.screen_reader = true;
            self.motion = MotionMode::Reduced;
            self.glyphs = GlyphSet::Ascii;
            self.revision += 1;
        }
    }

    pub fn with_motion(&mut self, motion: MotionMode) {
        self.motion = motion;
        self.revision += 1;
    }

    pub fn with_glyphs(&mut self, glyphs: GlyphSet) {
        self.glyphs = glyphs;
        self.revision += 1;
    }

    pub fn theme_revision(&self) -> u64 {
        self.revision
    }
}

impl Revisioned for Theme {
    fn revision(&self) -> u64 {
        self.revision
    }
}

#[cfg(test)]
mod tests {
    use super::{Palette, SemanticColor, Theme, contrast_ratio};

    #[test]
    fn high_contrast_palettes_meet_seven_to_one_for_semantic_text() {
        for palette in [
            Palette::high_contrast_dark(),
            Palette::high_contrast_light(),
        ] {
            for color in [
                palette.foreground,
                palette.accent,
                palette.success,
                palette.warning,
                palette.error,
                palette.muted,
                palette.border,
            ] {
                assert!(
                    contrast_ratio(color, palette.background).is_some_and(|ratio| ratio >= 7.0)
                );
            }
        }
    }

    #[test]
    fn custom_palette_reports_unmeasurable_terminal_indices() {
        let mut palette = Palette::dark_defaults();
        palette.muted = SemanticColor::Index(8);

        let report = palette.text_contrast_ratios();
        let muted = report
            .iter()
            .find(|entry| entry.role == super::PaletteTextRole::Muted)
            .expect("muted role is reported");
        assert_eq!(muted.ratio, None);
        assert_eq!(report.len(), 6);
        assert!(
            report
                .iter()
                .filter(|entry| entry.role != super::PaletteTextRole::Muted)
                .all(|entry| entry.ratio.is_some())
        );
    }

    #[test]
    fn screen_reader_profile_selects_static_ascii_output() {
        let mut theme = Theme::dark();
        let before = theme.theme_revision();
        theme.enable_screen_reader();

        assert!(theme.screen_reader);
        assert_eq!(theme.motion, super::MotionMode::Reduced);
        assert_eq!(theme.glyphs, super::GlyphSet::Ascii);
        assert!(theme.theme_revision() > before);
    }
}
