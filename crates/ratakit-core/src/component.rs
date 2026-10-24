use crate::{Event, Keymap, Outcome, Revisioned};

/// Shared pattern for stateful UI pieces: handle events, expose revision, return intents.
pub trait Component {
    type Intent;

    fn handle(&mut self, ev: &Event, keys: &Keymap) -> Outcome<Self::Intent>;

    fn revision(&self) -> u64;
}

impl<T: Component> Revisioned for T {
    fn revision(&self) -> u64 {
        Component::revision(self)
    }
}
