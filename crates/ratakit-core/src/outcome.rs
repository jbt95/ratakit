/// Result of feeding an [`Event`](crate::Event) into component state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome<I = ()> {
    /// The event was not handled; propagate to the next handler.
    Ignored,
    /// The event was handled; redraw if [`Revisioned::revision`](crate::Revisioned::revision) changed.
    Consumed,
    /// The event was handled and produced a high-level intent.
    Intent(I),
}

impl<I> Outcome<I> {
    pub fn is_consumed(&self) -> bool {
        !matches!(self, Self::Ignored)
    }
}
