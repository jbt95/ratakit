/// Result of feeding an [`Event`](crate::Event) into [`Component`](crate::Component) state.
///
/// `Ignored` allows event propagation. `Consumed` stops propagation without an
/// application-level result. `Intent` stops propagation and returns a value for
/// the application to interpret. Components do not execute the side effect
/// represented by an intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome<I = ()> {
    /// The event was not handled; propagate to the next handler.
    Ignored,
    /// The event was handled; redraw when the component's
    /// [`Revisioned::revision`](crate::Revisioned::revision) changes.
    Consumed,
    /// The event was handled and produced a high-level intent.
    Intent(I),
}

impl<I> Outcome<I> {
    pub fn is_consumed(&self) -> bool {
        !matches!(self, Self::Ignored)
    }
}
