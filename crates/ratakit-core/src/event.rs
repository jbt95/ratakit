use std::time::{Duration, Instant};

/// Normalized input events for ratakit components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Key(KeyEvent),
    Paste(String),
    Mouse(MouseEvent),
    Resize {
        width: u16,
        height: u16,
    },
    FocusGained,
    FocusLost,
    /// Optional animation tick (spinners, shimmer).
    Tick(Instant),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
    pub kind: KeyEventKind,
}

impl KeyEvent {
    pub fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
        }
    }

    pub fn with_modifiers(self, modifiers: KeyModifiers) -> Self {
        Self { modifiers, ..self }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyEventKind {
    Press,
    Repeat,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct KeyModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,
}

impl KeyModifiers {
    pub const NONE: Self = Self {
        shift: false,
        control: false,
        alt: false,
        super_key: false,
    };

    pub fn contains(&self, other: Self) -> bool {
        (!other.shift || self.shift)
            && (!other.control || self.control)
            && (!other.alt || self.alt)
            && (!other.super_key || self.super_key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char),
    Enter,
    Tab,
    Backspace,
    Delete,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub column: u16,
    pub row: u16,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    Down(MouseButton),
    Up(MouseButton),
    Drag(MouseButton),
    ScrollUp,
    ScrollDown,
    Moved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Convenience for tests and scripted event playback.
#[allow(dead_code)]
pub fn tick_after(start: Instant, delta: Duration) -> Event {
    Event::Tick(start + delta)
}
