//! Core state, events, keymaps, focus management, and theming for [ratakit](https://github.com/jbt95/ratakit).
//!
//! This crate deliberately has **no** dependency on ratatui so application state can be unit-tested
//! without a terminal.
//!
//! Text editing state lives in [`TextBuffer`] ([`Atom`]s, undo/redo, large-paste collapsing) and
//! [`PasteBurst`] for terminals without bracketed paste.

mod component;
mod event;
mod focus;
#[cfg(feature = "fuzzy")]
mod fuzzy;
mod keymap;
mod outcome;
mod revision;
mod text;
mod theme;

pub use component::Component;
pub use event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
pub use focus::{FocusId, FocusRing};
#[cfg(feature = "fuzzy")]
pub use fuzzy::{
    FuzzyDelimiterMode, FuzzyFilter, FuzzyOptions, FuzzyRankedMatch, normalize_match_indices,
};
pub use keymap::{ActionId, KeyChord, Keymap, KeymapPreset, Scope};
pub use outcome::Outcome;
pub use revision::Revisioned;
pub use text::{Atom, AtomKind, PasteBurst, Position, TextBuffer};
pub use theme::{GlyphSet, MotionMode, Palette, SemanticColor, SyntaxThemeId, Theme};
