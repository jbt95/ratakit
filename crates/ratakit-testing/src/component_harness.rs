use ratakit_core::{Component, Event, Keymap, Outcome};

use crate::KeySeq;

/// Drives a [`Component`] with test events and a caller-selected keymap.
///
/// The harness deliberately leaves application state and rendering with the test: inspect the
/// component through [`component`](Self::component) and render it with the snapshot helpers.
pub struct ComponentHarness<C> {
    component: C,
    keymap: Keymap,
}

impl<C: Component> ComponentHarness<C> {
    pub fn new(component: C, keymap: Keymap) -> Self {
        Self { component, keymap }
    }

    /// Send one event and return the component's outcome.
    pub fn send(&mut self, event: &Event) -> Outcome<C::Intent> {
        self.component.handle(event, &self.keymap)
    }

    /// Send every event in a parsed key sequence and collect outcomes in order.
    pub fn send_keys(&mut self, sequence: &KeySeq) -> Vec<Outcome<C::Intent>> {
        sequence
            .events()
            .iter()
            .map(|event| self.send(event))
            .collect()
    }

    pub fn component(&self) -> &C {
        &self.component
    }

    pub fn component_mut(&mut self) -> &mut C {
        &mut self.component
    }

    pub fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    pub fn keymap_mut(&mut self) -> &mut Keymap {
        &mut self.keymap
    }
}

#[cfg(test)]
mod tests {
    use ratakit_core::{Component, Event, KeyCode, Keymap, Outcome};

    use super::ComponentHarness;

    #[derive(Default)]
    struct Counter(usize);

    impl Component for Counter {
        type Intent = usize;

        fn handle(&mut self, event: &Event, _keys: &Keymap) -> Outcome<Self::Intent> {
            match event {
                Event::Key(key) if key.code == KeyCode::Char('x') => {
                    self.0 += 1;
                    Outcome::Intent(self.0)
                }
                _ => Outcome::Ignored,
            }
        }

        fn revision(&self) -> u64 {
            self.0 as u64
        }
    }

    #[test]
    fn drives_sequences_and_preserves_outcomes() {
        let mut harness = ComponentHarness::new(Counter::default(), Keymap::empty());
        let keys = crate::KeySeq::parse("x y x").expect("valid key sequence");

        assert_eq!(
            harness.send_keys(&keys),
            vec![Outcome::Intent(1), Outcome::Ignored, Outcome::Intent(2)]
        );
        assert_eq!(harness.component().0, 2);
    }
}
