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

impl Palette {
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
    revision: u64,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            palette: Palette::dark_defaults(),
            syntax: SyntaxThemeId::BASE16_MARDUK,
            motion: MotionMode::Full,
            glyphs: GlyphSet::Unicode,
            revision: 0,
        }
    }

    pub fn light() -> Self {
        Self {
            palette: Palette::light_defaults(),
            syntax: SyntaxThemeId::BASE16_MARDUK,
            motion: MotionMode::Full,
            glyphs: GlyphSet::Unicode,
            revision: 0,
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
