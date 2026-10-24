use crate::{
    Component, Event, KeyCode, KeyEventKind, MouseEventKind, Outcome, Revisioned, ScrollState,
};

/// Measures row heights for virtualized lists. Heights may depend on layout width and should be
/// stable until [`revision`](Self::revision) changes.
pub trait ItemHeights {
    /// Number of items in the list.
    fn len(&self) -> usize;

    /// Whether the list has no items.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Height of one item in terminal rows at the given width.
    fn height(&self, index: usize, width: u16) -> u16;

    /// Content revision used to invalidate height caches. Defaults to `0`.
    fn revision(&self) -> u64 {
        0
    }
}

/// Cached per-item heights and prefix sums for a fixed width and source revision.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeightCache {
    width: u16,
    source_revision: u64,
    item_count: usize,
    heights: Vec<u16>,
    /// Cumulative start row of each item in content coordinates.
    starts: Vec<usize>,
}

impl HeightCache {
    /// Rebuilds the cache when width, item count, or source revision changed.
    pub fn sync(&mut self, source: &impl ItemHeights, width: u16) {
        let len = source.len();
        let rev = source.revision();
        if self.width == width && self.source_revision == rev && self.item_count == len {
            return;
        }
        self.width = width;
        self.source_revision = rev;
        self.item_count = len;
        self.heights.clear();
        self.starts.clear();
        let mut y = 0usize;
        for index in 0..len {
            self.starts.push(y);
            let h = source.height(index, width).max(1);
            self.heights.push(h);
            y += h as usize;
        }
    }

    /// Total content height in terminal rows.
    #[must_use]
    pub fn total_height(&self) -> usize {
        if self.heights.is_empty() {
            return 0;
        }
        let last = self.heights.len() - 1;
        self.starts[last] + self.heights[last] as usize
    }

    /// Maps a content-row offset to `(item_index, skip_top_rows_within_item)`.
    #[must_use]
    pub fn locate(&self, offset: usize) -> (usize, u16) {
        if self.heights.is_empty() {
            return (0, 0);
        }
        let index = self
            .starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1);
        let index = index.min(self.heights.len() - 1);
        let skip = offset.saturating_sub(self.starts[index]);
        (index, skip.min(self.heights[index] as usize) as u16)
    }

    /// Height of `index` in rows (at the cache's width).
    #[must_use]
    pub fn item_height(&self, index: usize) -> u16 {
        self.heights.get(index).copied().unwrap_or(1)
    }

    /// Content-row offset where `index` starts.
    #[must_use]
    pub fn item_start(&self, index: usize) -> usize {
        self.starts.get(index).copied().unwrap_or(0)
    }
}

/// Describes which items intersect the viewport for one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleRows {
    pub first_index: usize,
    pub skip_top: u16,
    pub last_index: usize,
}

/// Computes the range of items that must be drawn for the current scroll offset.
#[must_use]
pub fn visible_rows(
    cache: &HeightCache,
    scroll_offset: usize,
    viewport_height: u16,
) -> VisibleRows {
    if cache.heights.is_empty() || viewport_height == 0 {
        return VisibleRows {
            first_index: 0,
            skip_top: 0,
            last_index: 0,
        };
    }
    let (first_index, skip_top) = cache.locate(scroll_offset);
    let bottom = scroll_offset + viewport_height as usize;
    let mut last_index = first_index;
    for index in first_index..cache.heights.len() {
        let start = cache.item_start(index);
        if start >= bottom {
            break;
        }
        last_index = index;
    }
    VisibleRows {
        first_index,
        skip_top,
        last_index,
    }
}

/// Last measured layout, updated by the virtual list widget each frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ListLayoutHint {
    pub viewport_height: u16,
    pub content_height: usize,
    pub width: u16,
}

/// Selection and scroll state for a virtualized list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListState {
    scroll: ScrollState,
    selected: Option<usize>,
    layout: ListLayoutHint,
    height_cache: HeightCache,
    revision: u64,
}

impl ListState {
    /// New list state with follow-tail enabled.
    #[must_use]
    pub fn new() -> Self {
        Self {
            scroll: ScrollState::new(),
            selected: None,
            layout: ListLayoutHint::default(),
            height_cache: HeightCache::default(),
            revision: 0,
        }
    }

    /// Scroll state (offset and follow-tail).
    #[must_use]
    pub fn scroll(&self) -> &ScrollState {
        &self.scroll
    }

    /// Mutable scroll state.
    pub fn scroll_mut(&mut self) -> &mut ScrollState {
        &mut self.scroll
    }

    /// Selected item index, if any.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Select an item (or clear with `None`).
    pub fn select(&mut self, index: Option<usize>) {
        if self.selected != index {
            self.selected = index;
            self.revision += 1;
        }
    }

    /// Last layout hint written by the list widget (for scroll key handling).
    #[must_use]
    pub fn layout(&self) -> ListLayoutHint {
        self.layout
    }

    /// Height measurements cached for the last layout width.
    #[must_use]
    pub fn height_cache(&self) -> &HeightCache {
        &self.height_cache
    }

    /// Sync height cache and scroll position for a new frame.
    pub fn prepare_layout(&mut self, source: &impl ItemHeights, width: u16, viewport_height: u16) {
        self.height_cache.sync(source, width);
        let content_height = self.height_cache.total_height();
        self.sync_scroll(width, viewport_height, content_height);
    }

    /// Update follow-tail and clamp offset after layout changes.
    pub fn sync_scroll(&mut self, width: u16, viewport_height: u16, content_height: usize) {
        self.layout = ListLayoutHint {
            width,
            viewport_height,
            content_height,
        };
        self.scroll.maintain_tail(viewport_height, content_height);
        self.scroll.clamp(viewport_height, content_height);
    }

    fn scroll_key(&mut self, delta: isize) {
        let hint = self.layout;
        if hint.viewport_height == 0 && hint.content_height == 0 {
            self.scroll.scroll_by_unbounded(delta);
        } else {
            self.scroll
                .scroll_by(delta, hint.viewport_height.max(1), hint.content_height);
        }
        self.bump();
    }

    fn bump(&mut self) {
        self.revision += 1;
    }
}

impl Component for ListState {
    type Intent = ();

    fn handle(&mut self, ev: &Event, _keys: &crate::Keymap) -> Outcome<()> {
        match ev {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Up => {
                    self.scroll_key(-1);
                    Outcome::Consumed
                }
                KeyCode::Down => {
                    self.scroll_key(1);
                    Outcome::Consumed
                }
                KeyCode::PageUp => {
                    let page = self.layout.viewport_height.max(1) as isize;
                    self.scroll_key(-page);
                    Outcome::Consumed
                }
                KeyCode::PageDown => {
                    let page = self.layout.viewport_height.max(1) as isize;
                    self.scroll_key(page);
                    Outcome::Consumed
                }
                KeyCode::Home => {
                    self.scroll.release_tail();
                    self.scroll.scroll_to_offset(0);
                    self.bump();
                    Outcome::Consumed
                }
                KeyCode::End => {
                    let hint = self.layout;
                    self.scroll
                        .scroll_to_end(hint.viewport_height.max(1), hint.content_height);
                    self.bump();
                    Outcome::Consumed
                }
                _ => Outcome::Ignored,
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_key(-3);
                    Outcome::Consumed
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_key(3);
                    Outcome::Consumed
                }
                _ => Outcome::Ignored,
            },
            _ => Outcome::Ignored,
        }
    }

    fn revision(&self) -> u64 {
        self.revision.max(self.scroll.revision())
    }
}
