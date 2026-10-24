use crate::Revisioned;
use crate::keymap::{ActionId, KeyChord, Keymap, Scope};

/// Human-readable description of a key binding for the status bar or help overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyHint {
    pub chord: String,
    pub action: ActionId,
    pub label: String,
}

/// Lookup table from [`ActionId`] to a short label shown in the UI.
#[derive(Debug, Clone, Default)]
pub struct ActionLabels {
    labels: std::collections::HashMap<ActionId, String>,
}

impl ActionLabels {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, action: impl Into<ActionId>, label: impl Into<String>) {
        self.labels.insert(action.into(), label.into());
    }

    pub fn label_for(&self, action: &ActionId) -> String {
        self.labels
            .get(action)
            .cloned()
            .unwrap_or_else(|| humanize_action_id(action))
    }
}

/// Format a [`KeyChord`] for display (e.g. `ctrl+enter`).
pub fn format_chord(chord: &KeyChord) -> String {
    use crate::event::KeyCode;

    let mut parts: Vec<String> = Vec::new();
    if chord.modifiers.control {
        parts.push("ctrl".into());
    }
    if chord.modifiers.alt {
        parts.push("alt".into());
    }
    if chord.modifiers.shift {
        parts.push("shift".into());
    }
    if chord.modifiers.super_key {
        parts.push("super".into());
    }

    let key = match chord.code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "enter".into(),
        KeyCode::Tab => "tab".into(),
        KeyCode::Backspace => "backspace".into(),
        KeyCode::Delete => "delete".into(),
        KeyCode::Escape => "esc".into(),
        KeyCode::Left => "left".into(),
        KeyCode::Right => "right".into(),
        KeyCode::Up => "up".into(),
        KeyCode::Down => "down".into(),
        KeyCode::Home => "home".into(),
        KeyCode::End => "end".into(),
        KeyCode::PageUp => "pageup".into(),
        KeyCode::PageDown => "pagedown".into(),
        KeyCode::F(n) => format!("f{n}"),
        KeyCode::Unknown => "?".into(),
    };
    parts.push(key);
    parts.join("+")
}

fn humanize_action_id(action: &ActionId) -> String {
    let last = action.0.rsplit('.').next().unwrap_or(&action.0);
    last.replace('_', " ")
}

/// Collect key hints from the keymap in resolution order (modal, focused, global).
///
/// At most `limit` hints are returned; duplicate actions are skipped (first binding wins).
pub fn key_hints_from_keymap(keymap: &Keymap, labels: &ActionLabels, limit: usize) -> Vec<KeyHint> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for scope in [Scope::Modal, Scope::Focused, Scope::Global] {
        for binding in keymap.bindings_in_scope(scope) {
            if seen.contains(&binding.action) {
                continue;
            }
            seen.insert(binding.action.clone());
            out.push(KeyHint {
                chord: format_chord(&binding.chord),
                action: binding.action.clone(),
                label: labels.label_for(&binding.action),
            });
            if out.len() >= limit {
                return out;
            }
        }
    }
    out
}

/// Revision token for cache keys when the keymap changes.
pub fn key_hints_revision(keymap: &Keymap) -> u64 {
    Revisioned::revision(keymap)
}
