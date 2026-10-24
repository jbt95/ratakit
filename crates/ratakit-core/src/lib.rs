//! Core state, events, keymaps, focus management, and theming for [ratakit](https://github.com/jbt95/ratakit).
//!
//! This crate deliberately has **no** dependency on ratatui so application state can be unit-tested
//! without a terminal.

mod component;
mod event;
mod focus;
mod key_hints;
mod keymap;
mod motion;
mod outcome;
mod revision;
mod theme;
mod toast;

pub use component::Component;
pub use event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
pub use focus::{FocusId, FocusRing};
pub use key_hints::{
    ActionLabels, KeyHint, format_chord, key_hints_from_keymap, key_hints_revision,
};
pub use keymap::{ActionId, KeyBinding, KeyChord, Keymap, KeymapPreset, Scope};
pub use motion::Motion;
pub use outcome::Outcome;
pub use revision::Revisioned;
pub use theme::{GlyphSet, MotionMode, Palette, SemanticColor, SyntaxThemeId, Theme};
pub use toast::{Toast, ToastLevel, ToastQueue};
