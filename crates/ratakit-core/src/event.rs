use std::time::{Duration, Instant};

/// Normalized terminal input and timer events for ratakit components.
///
/// Convert backend-specific events at the application edge, then route them to
/// the focused component or application handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Keyboard input.
    Key(KeyEvent),
    /// Text pasted into the terminal.
    Paste(String),
    /// Mouse input in terminal-cell coordinates.
    Mouse(MouseEvent),
    /// Terminal dimensions changed.
    Resize {
        /// New terminal width in columns.
        width: u16,
        /// New terminal height in rows.
        height: u16,
    },
    /// Terminal focus was gained.
    FocusGained,
    /// Terminal focus was lost.
    FocusLost,
    /// Optional animation tick (spinners, shimmer).
    Tick(Instant),
}

/// A key code, modifier set, and press/repeat/release phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    /// Logical key pressed, such as `Enter` or `Char('x')`.
    pub code: KeyCode,
    /// Modifier keys held with this key.
    pub modifiers: KeyModifiers,
    /// Whether this is a press, repeat, or release event.
    pub kind: KeyEventKind,
}

impl KeyEvent {
    /// Create an unmodified key-press event.
    pub fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
        }
    }

    /// Replace this event's modifier set.
    pub fn with_modifiers(self, modifiers: KeyModifiers) -> Self {
        Self { modifiers, ..self }
    }
}

/// Phase of a key event reported by the terminal backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyEventKind {
    /// A key was pressed.
    Press,
    /// A held key repeated.
    Repeat,
    /// A key was released.
    Release,
}

/// Set of modifier keys attached to a key event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct KeyModifiers {
    /// Shift is held.
    pub shift: bool,
    /// Control is held.
    pub control: bool,
    /// Alt/Option is held.
    pub alt: bool,
    /// Super/Command/Windows is held.
    pub super_key: bool,
}

impl KeyModifiers {
    /// No modifier keys are pressed.
    pub const NONE: Self = Self {
        shift: false,
        control: false,
        alt: false,
        super_key: false,
    };

    /// Whether every modifier required by `other` is present in `self`.
    pub fn contains(&self, other: Self) -> bool {
        (!other.shift || self.shift)
            && (!other.control || self.control)
            && (!other.alt || self.alt)
            && (!other.super_key || self.super_key)
    }
}

/// Logical key codes supported by the normalized event model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// A printable or otherwise character-valued key.
    Char(char),
    /// Enter/Return.
    Enter,
    /// Tab.
    Tab,
    /// Backspace.
    Backspace,
    /// Delete.
    Delete,
    /// Escape.
    Escape,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Home.
    Home,
    /// End.
    End,
    /// Page Up.
    PageUp,
    /// Page Down.
    PageDown,
    /// Function key by number.
    F(u8),
    /// Backend key not represented by this enum.
    Unknown,
}

/// Mouse event and its location in terminal-cell coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    /// Mouse action.
    pub kind: MouseEventKind,
    /// Zero-based column.
    pub column: u16,
    /// Zero-based row.
    pub row: u16,
    /// Modifier keys held during the action.
    pub modifiers: KeyModifiers,
}

/// Mouse actions represented by [`Event::Mouse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    /// A button was pressed.
    Down(MouseButton),
    /// A button was released.
    Up(MouseButton),
    /// A button was dragged.
    Drag(MouseButton),
    /// The wheel moved upward.
    ScrollUp,
    /// The wheel moved downward.
    ScrollDown,
    /// Pointer movement or an unsupported horizontal scroll action.
    Moved,
}

/// Mouse buttons supported by the normalized event model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// Primary button.
    Left,
    /// Secondary button.
    Right,
    /// Middle button.
    Middle,
}

/// Create an [`Event::Tick`] after `delta` has elapsed from `start`.
///
/// This helper is primarily useful in tests and scripted event playback.
#[allow(dead_code)]
pub fn tick_after(start: Instant, delta: Duration) -> Event {
    Event::Tick(start + delta)
}
