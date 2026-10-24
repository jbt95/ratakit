use crate::component::Component;
use crate::event::{Event, KeyCode, KeyEventKind, MouseEventKind};
use crate::keymap::Keymap;
use crate::outcome::Outcome;

/// Last measured layout for scroll key handling (set by scroll views).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScrollLayoutHint {
    pub viewport_height: u16,
    pub content_height: usize,
}

/// Vertical scroll offset for virtualized content, with optional *follow tail* during streaming.
///
/// The offset is measured in **terminal rows** (not item indices). When [`follow_tail`](Self::follow_tail)
/// is active, [`maintain_tail`](Self::maintain_tail) keeps the viewport pinned to the end as content grows.
/// Any upward scroll releases follow-tail until [`scroll_to_end`](Self::scroll_to_end) is called again.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScrollState {
    offset: usize,
    follow_tail: bool,
    layout: ScrollLayoutHint,
    revision: u64,
}

impl ScrollState {
    /// Creates a scroll state pinned to the top with follow-tail enabled.
    #[must_use]
    pub fn new() -> Self {
        Self {
            offset: 0,
            follow_tail: true,
            layout: ScrollLayoutHint::default(),
            revision: 0,
        }
    }

    /// Last layout hint written by a scroll view widget.
    #[must_use]
    pub fn layout(&self) -> ScrollLayoutHint {
        self.layout
    }

    /// Update follow-tail and clamp offset after layout changes.
    pub fn sync_layout(&mut self, viewport_height: u16, content_height: usize) {
        self.layout = ScrollLayoutHint {
            viewport_height,
            content_height,
        };
        self.maintain_tail(viewport_height, content_height);
        self.clamp(viewport_height, content_height);
    }

    /// Current scroll offset in terminal rows from the top of the content.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Whether the viewport should stay at the end when content grows.
    #[must_use]
    pub fn follow_tail(&self) -> bool {
        self.follow_tail
    }

    /// Scroll by `delta` rows (negative moves up). Scrolling up disables follow-tail.
    pub fn scroll_by(&mut self, delta: isize, viewport_height: u16, content_height: usize) {
        if delta < 0 {
            self.follow_tail = false;
        }
        let max_offset = max_scroll_offset(viewport_height, content_height);
        let next = self.offset.saturating_add_signed(delta);
        self.set_offset(next.min(max_offset));
    }

    /// Jump to the end and re-enable follow-tail.
    pub fn scroll_to_end(&mut self, viewport_height: u16, content_height: usize) {
        self.follow_tail = true;
        self.set_offset(max_scroll_offset(viewport_height, content_height));
    }

    /// If follow-tail is on, pin the offset to the end (call after content height changes).
    pub fn maintain_tail(&mut self, viewport_height: u16, content_height: usize) {
        if !self.follow_tail {
            return;
        }
        let end = max_scroll_offset(viewport_height, content_height);
        if self.offset != end {
            self.set_offset(end);
        }
    }

    /// Clamp offset when the viewport or content size changes.
    pub fn clamp(&mut self, viewport_height: u16, content_height: usize) {
        let max_offset = max_scroll_offset(viewport_height, content_height);
        if self.offset > max_offset {
            self.set_offset(max_offset);
        }
    }

    /// Sets the scroll offset in rows (disables follow-tail when moving upward).
    pub fn scroll_to_offset(&mut self, offset: usize) {
        if offset < self.offset {
            self.release_tail();
        }
        self.set_offset(offset);
    }

    /// Disables follow-tail without changing the offset (for example Home).
    pub fn release_tail(&mut self) {
        if self.follow_tail {
            self.follow_tail = false;
            self.revision += 1;
        }
    }

    /// Adjust offset without clamping (tests or when layout is not yet known).
    pub fn scroll_by_unbounded(&mut self, delta: isize) {
        if delta < 0 {
            self.release_tail();
        }
        let next = self.offset.saturating_add_signed(delta);
        self.set_offset(next);
    }

    fn set_offset(&mut self, offset: usize) {
        if self.offset != offset {
            self.offset = offset;
            self.revision += 1;
        }
    }
}

impl Component for ScrollState {
    type Intent = ();

    fn handle(&mut self, ev: &Event, _keys: &Keymap) -> Outcome<()> {
        let hint = self.layout;
        let viewport = hint.viewport_height.max(1);
        let content = hint.content_height;
        match ev {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Up => {
                    self.scroll_by(-1, viewport, content);
                    Outcome::Consumed
                }
                KeyCode::Down => {
                    self.scroll_by(1, viewport, content);
                    Outcome::Consumed
                }
                KeyCode::PageUp => {
                    self.scroll_by(-(viewport as isize), viewport, content);
                    Outcome::Consumed
                }
                KeyCode::PageDown => {
                    self.scroll_by(viewport as isize, viewport, content);
                    Outcome::Consumed
                }
                KeyCode::Home => {
                    self.release_tail();
                    self.scroll_to_offset(0);
                    Outcome::Consumed
                }
                KeyCode::End => {
                    self.scroll_to_end(viewport, content);
                    Outcome::Consumed
                }
                _ => Outcome::Ignored,
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_by(-3, viewport, content);
                    Outcome::Consumed
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_by(3, viewport, content);
                    Outcome::Consumed
                }
                _ => Outcome::Ignored,
            },
            _ => Outcome::Ignored,
        }
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}

#[must_use]
fn max_scroll_offset(viewport_height: u16, content_height: usize) -> usize {
    content_height.saturating_sub(viewport_height as usize)
}
