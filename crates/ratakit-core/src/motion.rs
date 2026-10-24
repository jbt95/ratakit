use crate::theme::{GlyphSet, MotionMode, Theme};
use crate::{Component, Event, Keymap, Outcome};

/// Animation frame state for spinners and similar indicators.
///
/// Advance with [`Event::Tick`] via [`Component::handle`] or [`Motion::advance`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Motion {
    frame: usize,
    mode: MotionMode,
    screen_reader: bool,
    revision: u64,
}

const SPINNER_UNICODE: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_ASCII: &[&str] = &["|", "/", "-", "\\"];

fn frame_count(glyphs: GlyphSet) -> usize {
    match glyphs {
        GlyphSet::Unicode => SPINNER_UNICODE.len(),
        GlyphSet::Ascii => SPINNER_ASCII.len(),
    }
}

impl Motion {
    /// New spinner motion in [`MotionMode::Full`].
    pub fn new() -> Self {
        Self {
            frame: 0,
            mode: MotionMode::Full,
            screen_reader: false,
            revision: 0,
        }
    }

    /// Current motion mode (usually mirrored from [`Theme::motion`]).
    pub fn mode(&self) -> MotionMode {
        self.mode
    }

    /// Update motion mode when the theme changes.
    pub fn set_mode(&mut self, mode: MotionMode) {
        if self.mode != mode {
            self.mode = mode;
            self.revision += 1;
        }
    }

    /// Synchronize screen-reader mode so tick handling remains static as well.
    pub fn set_screen_reader(&mut self, enabled: bool) {
        if self.screen_reader != enabled {
            self.screen_reader = enabled;
            if enabled {
                self.mode = MotionMode::Reduced;
            }
            self.revision += 1;
        }
    }

    /// Zero-based frame index (meaningful only in [`MotionMode::Full`]).
    pub fn frame(&self) -> usize {
        self.frame
    }

    /// Static label used when [`MotionMode::Reduced`] is active.
    pub fn reduced_label(&self) -> &'static str {
        "working…"
    }

    /// Glyph or label for the current theme and motion settings.
    pub fn indicator(&self, theme: &Theme) -> &str {
        if theme.screen_reader {
            return "Working";
        }
        if theme.motion == MotionMode::Reduced {
            return self.reduced_label();
        }
        let frames = match theme.glyphs {
            GlyphSet::Unicode => SPINNER_UNICODE,
            GlyphSet::Ascii => SPINNER_ASCII,
        };
        frames[self.frame % frames.len()]
    }

    /// Advance one animation step; returns `true` if [`Motion::revision`] changed.
    pub fn advance(&mut self, theme: &Theme) -> bool {
        if theme.screen_reader || theme.motion != MotionMode::Full {
            return false;
        }
        let count = frame_count(theme.glyphs);
        self.frame = (self.frame + 1) % count;
        self.revision += 1;
        true
    }
}

impl Default for Motion {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Motion {
    type Intent = ();

    fn handle(&mut self, ev: &Event, _keys: &Keymap) -> Outcome<Self::Intent> {
        match ev {
            Event::Tick(_) if self.mode == MotionMode::Full && !self.screen_reader => {
                // Frame count without theme: use the longer Unicode cycle so ticks stay in sync
                // when the widget renders with either glyph set.
                self.frame = (self.frame + 1) % SPINNER_UNICODE.len();
                self.revision += 1;
                Outcome::Consumed
            }
            _ => Outcome::Ignored,
        }
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}
