use crate::Revisioned;

/// Opaque focus target (pane, field, modal control, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusId(pub u32);

impl FocusId {
    pub const ROOT: Self = Self(0);
}

/// Ordered focus ring with modal traps (stack).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusRing {
    order: Vec<FocusId>,
    index: usize,
    traps: Vec<FocusId>,
    revision: u64,
}

impl FocusRing {
    pub fn new(order: impl IntoIterator<Item = FocusId>) -> Self {
        Self {
            order: order.into_iter().collect(),
            index: 0,
            traps: Vec::new(),
            revision: 0,
        }
    }

    pub fn focused(&self) -> Option<FocusId> {
        if let Some(id) = self.traps.last() {
            return Some(*id);
        }
        self.order.get(self.index).copied()
    }

    pub fn contains(&self, id: FocusId) -> bool {
        self.order.contains(&id)
    }

    pub fn next(&mut self) {
        if self.order.is_empty() {
            return;
        }
        self.index = (self.index + 1) % self.order.len();
        self.bump();
    }

    pub fn prev(&mut self) {
        if self.order.is_empty() {
            return;
        }
        let len = self.order.len();
        self.index = (self.index + len - 1) % len;
        self.bump();
    }

    pub fn set_focus(&mut self, id: FocusId) {
        if let Some(pos) = self.order.iter().position(|&x| x == id) {
            self.index = pos;
            self.bump();
        }
    }

    /// Push a focus trap (e.g. when a modal opens). Focus stays on `id` until [`Self::pop_trap`].
    pub fn push_trap(&mut self, id: FocusId) {
        self.traps.push(id);
        self.bump();
    }

    pub fn pop_trap(&mut self) {
        if self.traps.pop().is_some() {
            self.bump();
        }
    }

    pub fn trap_depth(&self) -> usize {
        self.traps.len()
    }

    fn bump(&mut self) {
        self.revision += 1;
    }
}

impl Revisioned for FocusRing {
    fn revision(&self) -> u64 {
        self.revision
    }
}
