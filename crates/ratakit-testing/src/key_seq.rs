use ratakit_core::{Event, KeyCode, KeyEvent, KeyModifiers};

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeySeqError {
    UnknownToken(String),
    EmptyChord,
}

impl fmt::Display for KeySeqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownToken(t) => write!(f, "unknown key token: {t}"),
            Self::EmptyChord => write!(f, "empty key sequence"),
        }
    }
}

impl std::error::Error for KeySeqError {}

/// Parsed key sequence for driving state machines in tests.
#[derive(Debug, Clone, Default)]
pub struct KeySeq {
    events: Vec<Event>,
}

impl KeySeq {
    pub fn parse(input: &str) -> Result<Self, KeySeqError> {
        let mut events = Vec::new();
        for token in input.split_whitespace() {
            let key = parse_token(token)?;
            events.push(Event::Key(key));
        }
        if events.is_empty() {
            return Err(KeySeqError::EmptyChord);
        }
        Ok(Self { events })
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn into_events(self) -> Vec<Event> {
        self.events
    }
}

impl FromStr for KeySeq {
    type Err = KeySeqError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

fn parse_token(token: &str) -> Result<KeyEvent, KeySeqError> {
    let lower = token.to_ascii_lowercase();
    let parts: Vec<&str> = lower.split('+').collect();
    let mut modifiers = KeyModifiers::NONE;
    let key_part = if parts.len() == 1 {
        parts[0]
    } else {
        for part in &parts[..parts.len() - 1] {
            match *part {
                "ctrl" | "control" => modifiers.control = true,
                "alt" => modifiers.alt = true,
                "shift" => modifiers.shift = true,
                "super" | "cmd" | "meta" => modifiers.super_key = true,
                other => return Err(KeySeqError::UnknownToken(other.to_string())),
            }
        }
        parts[parts.len() - 1]
    };

    let code = match key_part {
        "enter" | "return" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "esc" | "escape" => KeyCode::Escape,
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        s if s.len() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        s => {
            if let Some(rest) = s.strip_prefix('f') {
                let n: u8 = rest
                    .parse()
                    .map_err(|_| KeySeqError::UnknownToken(token.to_string()))?;
                KeyCode::F(n)
            } else {
                return Err(KeySeqError::UnknownToken(s.to_string()));
            }
        }
    };

    Ok(KeyEvent::new(code).with_modifiers(modifiers))
}
