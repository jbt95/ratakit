use std::collections::HashMap;

use crate::Revisioned;
use crate::event::{KeyCode, KeyEvent, KeyModifiers};

/// Named action (e.g. `"composer.submit"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActionId(pub String);

impl ActionId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl From<&str> for ActionId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Binding scope; resolution order is `Modal > Focused > Global`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Global,
    Focused,
    Modal,
}

/// A single keymap entry with its scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBinding {
    pub scope: Scope,
    pub chord: KeyChord,
    pub action: ActionId,
}

/// Key combination used in a keymap layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyChord {
    pub fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    pub fn with_modifiers(self, modifiers: KeyModifiers) -> Self {
        Self { modifiers, ..self }
    }

    pub fn matches(&self, key: &KeyEvent) -> bool {
        self.code == key.code && self.modifiers == key.modifiers
    }
}

#[derive(Debug, Clone)]
struct Layer {
    bindings: HashMap<KeyChord, ActionId>,
}

impl Layer {
    fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }
}

/// Multi-layer keymap with named actions.
#[derive(Debug, Clone)]
pub struct Keymap {
    global: Layer,
    focused: Layer,
    modal: Layer,
    revision: u64,
}

impl Keymap {
    pub fn empty() -> Self {
        Self {
            global: Layer::new(),
            focused: Layer::new(),
            modal: Layer::new(),
            revision: 0,
        }
    }

    pub fn bind(&mut self, scope: Scope, chord: KeyChord, action: ActionId) {
        let layer = self.layer_mut(scope);
        layer.bindings.insert(chord, action);
        self.revision += 1;
    }

    pub fn resolve(&self, scopes: &[Scope], key: &KeyEvent) -> Option<&ActionId> {
        for scope in scopes {
            let layer = self.layer(*scope);
            for (chord, action) in &layer.bindings {
                if chord.matches(key) {
                    return Some(action);
                }
            }
        }
        None
    }

    pub fn resolve_default(&self, key: &KeyEvent) -> Option<&ActionId> {
        self.resolve(&[Scope::Modal, Scope::Focused, Scope::Global], key)
    }

    /// All bindings in a single scope layer.
    pub fn bindings_in_scope(&self, scope: Scope) -> Vec<KeyBinding> {
        let layer = self.layer(scope);
        let mut bindings: Vec<KeyBinding> = layer
            .bindings
            .iter()
            .map(|(chord, action)| KeyBinding {
                scope,
                chord: *chord,
                action: action.clone(),
            })
            .collect();
        bindings.sort_by(|a, b| a.action.0.cmp(&b.action.0));
        bindings
    }

    fn layer(&self, scope: Scope) -> &Layer {
        match scope {
            Scope::Global => &self.global,
            Scope::Focused => &self.focused,
            Scope::Modal => &self.modal,
        }
    }

    fn layer_mut(&mut self, scope: Scope) -> &mut Layer {
        match scope {
            Scope::Global => &mut self.global,
            Scope::Focused => &mut self.focused,
            Scope::Modal => &mut self.modal,
        }
    }
}

impl Revisioned for Keymap {
    fn revision(&self) -> u64 {
        self.revision
    }
}

/// Built-in keymap presets (minimal for milestone 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeymapPreset {
    Default,
    Emacs,
    VimLite,
}

impl Keymap {
    pub fn from_preset(preset: KeymapPreset) -> Self {
        let mut map = Self::empty();
        match preset {
            KeymapPreset::Default => {
                map.bind(
                    Scope::Global,
                    KeyChord::new(KeyCode::Tab),
                    ActionId::new("focus.next"),
                );
                map.bind(
                    Scope::Global,
                    KeyChord::new(KeyCode::Tab).with_modifiers(KeyModifiers {
                        shift: true,
                        ..KeyModifiers::NONE
                    }),
                    ActionId::new("focus.prev"),
                );
            }
            KeymapPreset::Emacs => {
                map.bind(
                    Scope::Global,
                    KeyChord::new(KeyCode::Char('x')).with_modifiers(KeyModifiers {
                        control: true,
                        ..KeyModifiers::NONE
                    }),
                    ActionId::new("app.cancel"),
                );
            }
            KeymapPreset::VimLite => {
                map.bind(
                    Scope::Focused,
                    KeyChord::new(KeyCode::Char('i')),
                    ActionId::new("composer.insert"),
                );
                map.bind(
                    Scope::Focused,
                    KeyChord::new(KeyCode::Escape),
                    ActionId::new("composer.normal"),
                );
            }
        }
        map
    }
}
