use ratakit_core::{HeightCache, ItemHeights, ListState, ScrollState, visible_rows};

struct FixedHeights {
    heights: Vec<u16>,
    revision: u64,
}

impl ItemHeights for FixedHeights {
    fn len(&self) -> usize {
        self.heights.len()
    }

    fn height(&self, index: usize, _width: u16) -> u16 {
        self.heights[index]
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}

#[test]
fn scroll_follow_tail_stays_at_end_when_content_grows() {
    let mut scroll = ScrollState::new();
    scroll.maintain_tail(3, 5);
    assert_eq!(scroll.offset(), 2);

    scroll.maintain_tail(3, 10);
    assert_eq!(scroll.offset(), 7);
    assert!(scroll.follow_tail());
}

#[test]
fn scroll_up_releases_follow_tail() {
    let mut scroll = ScrollState::new();
    scroll.scroll_to_end(3, 10);
    assert!(scroll.follow_tail());

    scroll.scroll_by(-1, 3, 10);
    assert!(!scroll.follow_tail());
    assert_eq!(scroll.offset(), 6);
}

#[test]
fn height_cache_and_visible_rows() {
    let source = FixedHeights {
        heights: vec![2, 1, 3],
        revision: 1,
    };
    let mut cache = HeightCache::default();
    cache.sync(&source, 40);
    assert_eq!(cache.total_height(), 6);

    let visible = visible_rows(&cache, 1, 3);
    assert_eq!(visible.first_index, 0);
    assert_eq!(visible.skip_top, 1);
    assert_eq!(visible.last_index, 2);
}

#[test]
fn list_prepare_layout_pins_tail_on_stream() {
    let mut source = FixedHeights {
        heights: vec![1, 1, 1],
        revision: 0,
    };
    let mut state = ListState::new();
    state.prepare_layout(&source, 20, 2);
    assert_eq!(state.scroll().offset(), 1);

    source.revision = 1;
    source.heights.push(1);
    state.prepare_layout(&source, 20, 2);
    assert_eq!(state.scroll().offset(), 2);
    assert!(state.scroll().follow_tail());
}

#[test]
fn scroll_to_item_selects_and_releases_follow_tail() {
    let source = FixedHeights {
        heights: vec![1, 3, 2],
        revision: 1,
    };
    let mut state = ListState::new();
    assert!(!state.scroll_to_item(1), "items need a measured layout");

    state.prepare_layout(&source, 20, 3);
    assert!(state.scroll().follow_tail());
    assert!(state.scroll_to_item(1));

    assert_eq!(state.selected(), Some(1));
    assert_eq!(state.scroll().offset(), 1);
    assert!(!state.scroll().follow_tail());
}
