use crate::{Event, Keymap, Outcome, Revisioned};

/// Event-driven state interface for ratakit components.
///
/// Implementations update their own interaction state and return an [`Outcome`];
/// they do not perform application side effects. The application chooses which
/// component receives each event and interprets any returned intent.
pub trait Component {
    /// Application-level result produced by handling an event.
    type Intent;

    /// Handle one normalized event using the application's key bindings.
    ///
    /// Return [`Outcome::Ignored`] to let the application try another handler.
    fn handle(&mut self, ev: &Event, keys: &Keymap) -> Outcome<Self::Intent>;

    /// Monotonic revision for render-visible state, used by caches and redraw logic.
    fn revision(&self) -> u64;
}

impl<T: Component> Revisioned for T {
    fn revision(&self) -> u64 {
        Component::revision(self)
    }
}
