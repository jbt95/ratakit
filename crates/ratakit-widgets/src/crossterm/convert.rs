//! Map [`crossterm::event::Event`] into [`ratakit_core::Event`].

use ratakit_core::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

/// Converts a crossterm event into the ratakit normalized representation.
pub fn from_crossterm_event(event: crossterm::event::Event) -> Event {
    match event {
        crossterm::event::Event::Key(key) => Event::Key(from_crossterm_key(key)),
        crossterm::event::Event::Mouse(mouse) => Event::Mouse(from_crossterm_mouse(mouse)),
        crossterm::event::Event::Resize(width, height) => Event::Resize { width, height },
        crossterm::event::Event::Paste(text) => Event::Paste(text),
        crossterm::event::Event::FocusGained => Event::FocusGained,
        crossterm::event::Event::FocusLost => Event::FocusLost,
    }
}

fn from_crossterm_key(key: crossterm::event::KeyEvent) -> KeyEvent {
    KeyEvent {
        code: from_crossterm_key_code(key.code),
        modifiers: from_crossterm_modifiers(key.modifiers),
        kind: from_crossterm_key_kind(key.kind),
    }
}

fn from_crossterm_key_kind(kind: crossterm::event::KeyEventKind) -> KeyEventKind {
    match kind {
        crossterm::event::KeyEventKind::Press => KeyEventKind::Press,
        crossterm::event::KeyEventKind::Repeat => KeyEventKind::Repeat,
        crossterm::event::KeyEventKind::Release => KeyEventKind::Release,
    }
}

fn from_crossterm_modifiers(modifiers: crossterm::event::KeyModifiers) -> KeyModifiers {
    KeyModifiers {
        shift: modifiers.contains(crossterm::event::KeyModifiers::SHIFT),
        control: modifiers.contains(crossterm::event::KeyModifiers::CONTROL),
        alt: modifiers.contains(crossterm::event::KeyModifiers::ALT),
        super_key: modifiers.contains(crossterm::event::KeyModifiers::SUPER),
    }
}

fn from_crossterm_key_code(code: crossterm::event::KeyCode) -> KeyCode {
    match code {
        crossterm::event::KeyCode::Char(c) => KeyCode::Char(c),
        crossterm::event::KeyCode::Enter => KeyCode::Enter,
        crossterm::event::KeyCode::Tab => KeyCode::Tab,
        crossterm::event::KeyCode::Backspace => KeyCode::Backspace,
        crossterm::event::KeyCode::Delete => KeyCode::Delete,
        crossterm::event::KeyCode::Esc => KeyCode::Escape,
        crossterm::event::KeyCode::Left => KeyCode::Left,
        crossterm::event::KeyCode::Right => KeyCode::Right,
        crossterm::event::KeyCode::Up => KeyCode::Up,
        crossterm::event::KeyCode::Down => KeyCode::Down,
        crossterm::event::KeyCode::Home => KeyCode::Home,
        crossterm::event::KeyCode::End => KeyCode::End,
        crossterm::event::KeyCode::PageUp => KeyCode::PageUp,
        crossterm::event::KeyCode::PageDown => KeyCode::PageDown,
        crossterm::event::KeyCode::F(n) => KeyCode::F(n),
        crossterm::event::KeyCode::Null
        | crossterm::event::KeyCode::BackTab
        | crossterm::event::KeyCode::Insert
        | crossterm::event::KeyCode::CapsLock
        | crossterm::event::KeyCode::NumLock
        | crossterm::event::KeyCode::ScrollLock
        | crossterm::event::KeyCode::PrintScreen
        | crossterm::event::KeyCode::Pause
        | crossterm::event::KeyCode::Menu
        | crossterm::event::KeyCode::KeypadBegin
        | crossterm::event::KeyCode::Media(_)
        | crossterm::event::KeyCode::Modifier(_) => KeyCode::Unknown,
    }
}

fn from_crossterm_mouse(mouse: crossterm::event::MouseEvent) -> MouseEvent {
    MouseEvent {
        kind: from_crossterm_mouse_kind(mouse.kind),
        column: mouse.column,
        row: mouse.row,
        modifiers: from_crossterm_modifiers(mouse.modifiers),
    }
}

fn from_crossterm_mouse_kind(kind: crossterm::event::MouseEventKind) -> MouseEventKind {
    match kind {
        crossterm::event::MouseEventKind::Down(button) => {
            MouseEventKind::Down(from_crossterm_mouse_button(button))
        }
        crossterm::event::MouseEventKind::Up(button) => {
            MouseEventKind::Up(from_crossterm_mouse_button(button))
        }
        crossterm::event::MouseEventKind::Drag(button) => {
            MouseEventKind::Drag(from_crossterm_mouse_button(button))
        }
        crossterm::event::MouseEventKind::ScrollUp => MouseEventKind::ScrollUp,
        crossterm::event::MouseEventKind::ScrollDown => MouseEventKind::ScrollDown,
        crossterm::event::MouseEventKind::ScrollLeft
        | crossterm::event::MouseEventKind::ScrollRight
        | crossterm::event::MouseEventKind::Moved => MouseEventKind::Moved,
    }
}

fn from_crossterm_mouse_button(button: crossterm::event::MouseButton) -> MouseButton {
    match button {
        crossterm::event::MouseButton::Left => MouseButton::Left,
        crossterm::event::MouseButton::Right => MouseButton::Right,
        crossterm::event::MouseButton::Middle => MouseButton::Middle,
    }
}
