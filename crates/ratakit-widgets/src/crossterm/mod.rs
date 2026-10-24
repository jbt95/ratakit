//! Crossterm integration: normalized events and terminal setup helpers.

mod convert;
mod terminal;

pub use convert::from_crossterm_event;
pub use terminal::{TerminalSession, TerminalViewport, enter_terminal, enter_terminal_with};
