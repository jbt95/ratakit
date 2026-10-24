//! Core state, events, keymaps, focus management, and theming for [ratakit](https://github.com/jbt95/ratakit).
//!
//! This crate deliberately has **no** dependency on ratatui so application state can be unit-tested
//! without a terminal.

mod component;
mod event;
mod focus;
mod keymap;
mod list;
mod outcome;
mod revision;
mod scroll;
mod theme;

pub use component::Component;
pub use event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
pub use focus::{FocusId, FocusRing};
pub use keymap::{ActionId, KeyChord, Keymap, KeymapPreset, Scope};
pub use list::{HeightCache, ItemHeights, ListLayoutHint, ListState, VisibleRows, visible_rows};
pub use outcome::Outcome;
pub use revision::Revisioned;
pub use scroll::ScrollState;
pub use theme::{GlyphSet, MotionMode, Palette, SemanticColor, SyntaxThemeId, Theme};
