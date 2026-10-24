use std::time::{Duration, Instant};

use crate::{Component, Event, Keymap, Outcome};

/// Severity for a toast notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// A single non-modal notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    expires_at: Option<Instant>,
}

impl Toast {
    /// Create a toast that expires after `ttl` from `now`.
    pub fn new(message: impl Into<String>, level: ToastLevel, now: Instant, ttl: Duration) -> Self {
        Self {
            message: message.into(),
            level,
            expires_at: Some(now + ttl),
        }
    }

    /// Toast that stays until dismissed manually.
    pub fn persistent(message: impl Into<String>, level: ToastLevel) -> Self {
        Self {
            message: message.into(),
            level,
            expires_at: None,
        }
    }

    pub fn is_expired(&self, now: Instant) -> bool {
        self.expires_at.is_some_and(|t| now >= t)
    }
}

/// Stackable toast queue; does not capture focus.
#[derive(Debug, Clone, Default)]
pub struct ToastQueue {
    toasts: Vec<Toast>,
    revision: u64,
}

impl ToastQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.toasts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    pub fn toasts(&self) -> &[Toast] {
        &self.toasts
    }

    /// Push a toast to the top of the stack (most recent last).
    pub fn push(&mut self, toast: Toast) {
        self.toasts.push(toast);
        self.revision += 1;
    }

    /// Remove expired toasts; returns whether anything was removed.
    pub fn expire(&mut self, now: Instant) -> bool {
        let before = self.toasts.len();
        self.toasts.retain(|t| !t.is_expired(now));
        let changed = self.toasts.len() != before;
        if changed {
            self.revision += 1;
        }
        changed
    }

    pub fn clear(&mut self) {
        if !self.toasts.is_empty() {
            self.toasts.clear();
            self.revision += 1;
        }
    }
}

impl Component for ToastQueue {
    type Intent = ();

    fn handle(&mut self, ev: &Event, _keys: &Keymap) -> Outcome<Self::Intent> {
        match ev {
            Event::Tick(instant) => {
                if self.expire(*instant) {
                    Outcome::Consumed
                } else {
                    Outcome::Ignored
                }
            }
            _ => Outcome::Ignored,
        }
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}
