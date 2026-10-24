// Inspired by openai/codex codex-rs/tui/src/bottom_pane/paste_burst.rs (Apache-2.0).
// Milestone 1 implementation is original to ratakit; see THIRD_PARTY.md when porting Codex sources.

use std::time::{Duration, Instant};

/// Detects rapid multi-character input (paste without bracketed paste) so Enter inserts
/// a newline instead of submitting.
#[derive(Debug, Clone)]
pub struct PasteBurst {
    rapid_chars: u32,
    active: bool,
    last_input: Option<Instant>,
    idle_timeout: Duration,
}

impl Default for PasteBurst {
    fn default() -> Self {
        Self::new()
    }
}

impl PasteBurst {
    pub fn new() -> Self {
        Self {
            rapid_chars: 0,
            active: false,
            last_input: None,
            idle_timeout: Duration::from_millis(500),
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn on_paste(&mut self, now: Instant) {
        self.active = true;
        self.rapid_chars = 0;
        self.last_input = Some(now);
    }

    pub fn on_char(&mut self, now: Instant) {
        self.rapid_chars += 1;
        if self.rapid_chars >= 4 {
            self.active = true;
        }
        self.last_input = Some(now);
    }

    pub fn on_other_key(&mut self, now: Instant) {
        self.last_input = Some(now);
        if !self.active {
            self.rapid_chars = 0;
        }
    }

    /// Returns `true` when Enter should insert `\n` instead of submitting.
    pub fn enter_inserts_newline(&mut self, now: Instant) -> bool {
        if !self.active {
            return false;
        }
        let still_hot = self
            .last_input
            .is_some_and(|t| now.duration_since(t) <= self.idle_timeout);
        if still_hot {
            return true;
        }
        self.reset();
        false
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.rapid_chars = 0;
        self.last_input = None;
    }

    pub fn tick(&mut self, now: Instant) {
        if self.active
            && self
                .last_input
                .is_some_and(|t| now.duration_since(t) > self.idle_timeout)
        {
            self.reset();
        }
    }
}
