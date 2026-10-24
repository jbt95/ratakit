//! Core state, events, keymaps, focus management, and theming for [ratakit](https://github.com/jbt95/ratakit).
//!
//! This crate deliberately has **no** dependency on ratatui so application state can be unit-tested
//! without a terminal.
//!
//! Text editing state lives in [`TextBuffer`] ([`Atom`]s, undo/redo, large-paste collapsing) and
//! [`PasteBurst`] for terminals without bracketed paste.

mod component;
mod dialog;
mod event;
mod focus;
#[cfg(feature = "fuzzy")]
mod fuzzy;
mod key_hints;
mod keymap;
mod list;
mod motion;
mod outcome;
mod revision;
mod scroll;
mod table;
mod tabs;
mod text;
mod theme;
mod toast;
mod tree;

pub use component::Component;
pub use dialog::{DialogIntent, DialogState};
pub use event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
pub use focus::{FocusId, FocusRing};
#[cfg(feature = "fuzzy")]
pub use fuzzy::{
    FuzzyDelimiterMode, FuzzyFilter, FuzzyOptions, FuzzyRankedMatch, normalize_match_indices,
};
pub use key_hints::{
    ActionLabels, KeyHint, format_chord, key_hints_from_keymap, key_hints_revision,
};
pub use keymap::{ActionId, KeyBinding, KeyChord, Keymap, KeymapPreset, Scope};
pub use list::{HeightCache, ItemHeights, ListLayoutHint, ListState, VisibleRows, visible_rows};
pub use motion::Motion;
pub use outcome::Outcome;
pub use revision::Revisioned;
pub use scroll::{ScrollLayoutHint, ScrollState};
pub use table::{SortDirection, TableState};
pub use tabs::TabsState;
pub use text::{Atom, AtomKind, PasteBurst, Position, TextBuffer};
pub use theme::{GlyphSet, MotionMode, Palette, Rgb, SemanticColor, SyntaxThemeId, Theme};
pub use toast::{Toast, ToastLevel, ToastQueue};
pub use tree::{
    FlatTreeRow, TreeIntent, TreePath, TreeRowSizing, TreeSource, TreeState, tree_indent_cols,
    tree_row_height,
};
