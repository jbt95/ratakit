use std::collections::HashSet;
use std::hash::Hash;

#[cfg(feature = "fuzzy")]
use crate::FuzzyFilter;
use crate::{Component, Event, KeyCode, KeyEventKind, Keymap, ListState, Outcome};

/// Path from the tree root to a node (`TreeItem` identifiers).
pub type TreePath<Id> = Vec<Id>;

/// How each visible tree row maps to terminal height when flattened for [`ListState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TreeRowSizing {
    /// One terminal row per node (default).
    #[default]
    Fullscreen,
    /// Row height follows wrapped label text at the current width.
    Content,
}

/// Application-provided tree data (lazy children are loaded out of band).
pub trait TreeSource<Id> {
    /// Number of children under `parent` (empty slice = roots).
    fn child_count(&self, parent: &[Id]) -> usize;

    /// Child identifier at `index` under `parent`.
    fn child_at(&self, parent: &[Id], index: usize) -> Id;

    /// Label drawn for the node at `path`.
    fn label(&self, path: &[Id]) -> &str;

    /// Whether the node can be expanded (may be true before children are loaded).
    fn is_branch(&self, path: &[Id]) -> bool;

    /// Whether [`child_count`](Self::child_count) reflects loaded children for `path`.
    fn children_loaded(&self, path: &[Id]) -> bool;

    /// Bumps when the tree data changes (invalidates flatten caches).
    fn revision(&self) -> u64 {
        0
    }
}

/// High-level result from [`TreeState::handle`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeIntent<Id> {
    /// The UI expanded a branch whose children are not loaded yet.
    LoadChildren(TreePath<Id>),
}

/// One flattened, visible row in the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatTreeRow<Id> {
    pub path: TreePath<Id>,
    pub depth: usize,
    pub is_branch: bool,
    pub is_open: bool,
    pub children_loaded: bool,
    pub is_loading_placeholder: bool,
}

/// Selection, expansion, filtering, and scroll for a hierarchical tree.
#[derive(Debug)]
pub struct TreeState<Id> {
    opened: HashSet<TreePath<Id>>,
    selected: Option<TreePath<Id>>,
    flat: Vec<FlatTreeRow<Id>>,
    list: ListState,
    row_sizing: TreeRowSizing,
    filter: String,
    #[cfg(feature = "fuzzy")]
    fuzzy: FuzzyFilter,
    pending_load: Option<TreePath<Id>>,
    source_revision: u64,
    layout_width: u16,
    revision: u64,
}

impl<Id> Default for TreeState<Id> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Id> TreeState<Id> {
    /// New tree with follow-tail scrolling and fullscreen row sizing.
    #[must_use]
    pub fn new() -> Self {
        Self {
            opened: HashSet::new(),
            selected: None,
            flat: Vec::new(),
            list: ListState::new(),
            row_sizing: TreeRowSizing::default(),
            filter: String::new(),
            #[cfg(feature = "fuzzy")]
            fuzzy: FuzzyFilter::new(),
            pending_load: None,
            source_revision: 0,
            layout_width: 0,
            revision: 0,
        }
    }

    /// Row height mode for the flattened list.
    #[must_use]
    pub fn row_sizing(&self) -> TreeRowSizing {
        self.row_sizing
    }

    /// Sets row height mode.
    pub fn set_row_sizing(&mut self, sizing: TreeRowSizing) {
        if self.row_sizing != sizing {
            self.row_sizing = sizing;
            self.revision += 1;
        }
    }

    fn bump(&mut self) {
        self.revision += 1;
    }
}

impl<Id: Clone + PartialEq + Eq + Hash> TreeState<Id> {
    /// Currently selected node path, if any.
    #[must_use]
    pub fn selected(&self) -> Option<&[Id]> {
        self.selected.as_deref()
    }

    /// Paths of expanded branch nodes.
    #[must_use]
    pub fn opened(&self) -> &HashSet<TreePath<Id>> {
        &self.opened
    }

    /// Flattened visible rows from the last [`rebuild_flat`](Self::rebuild_flat) call.
    #[must_use]
    pub fn flat_rows(&self) -> &[FlatTreeRow<Id>] {
        &self.flat
    }

    /// Underlying list scroll/selection state used by the tree widget.
    #[must_use]
    pub fn list(&self) -> &ListState {
        &self.list
    }

    /// Mutable list state (scroll offset).
    pub fn list_mut(&mut self) -> &mut ListState {
        &mut self.list
    }

    /// Filter query (plain substring when `fuzzy` is disabled).
    #[must_use]
    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// Sets the filter string and invalidates the flattened view.
    pub fn set_filter(&mut self, query: &str) {
        if self.filter == query {
            return;
        }
        self.filter = query.to_owned();
        #[cfg(feature = "fuzzy")]
        self.fuzzy.set_pattern(query);
        self.bump();
    }

    /// Takes a pending lazy-load request produced by expand/toggle.
    pub fn take_pending_load(&mut self) -> Option<TreePath<Id>> {
        self.pending_load.take()
    }

    /// Opens `path`. Returns `true` when the node was previously closed.
    pub fn open(&mut self, path: TreePath<Id>) -> bool {
        if path.is_empty() {
            return false;
        }
        if self.opened.insert(path) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Closes `path`. Returns `true` when the node was previously open.
    pub fn close(&mut self, path: &[Id]) -> bool {
        if path.is_empty() {
            return false;
        }
        if self.opened.remove(path) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Toggles `path`, requesting children when expanding an unloaded branch.
    pub fn toggle(&mut self, path: TreePath<Id>, source: &impl TreeSource<Id>) -> bool {
        if path.is_empty() {
            return false;
        }
        if self.is_open(&path) {
            return self.close(&path);
        }
        if source.is_branch(&path) && !source.children_loaded(&path) {
            self.pending_load = Some(path.clone());
            self.bump();
            return true;
        }
        self.open(path)
    }

    /// Toggles the currently selected branch.
    pub fn toggle_selected(&mut self, source: &impl TreeSource<Id>) -> bool {
        match self.selected.clone() {
            Some(path) => self.toggle(path, source),
            None => false,
        }
    }

    /// Selects `path`. Returns `true` when selection changed.
    pub fn select(&mut self, path: TreePath<Id>) -> bool {
        if path.is_empty() {
            if self.selected.is_some() {
                self.selected = None;
                self.list.select(None);
                self.bump();
                return true;
            }
            return false;
        }
        if self.selected.as_ref() == Some(&path) {
            return false;
        }
        self.selected = Some(path);
        self.sync_list_selection();
        self.bump();
        true
    }

    /// Clears selection.
    pub fn clear_selection(&mut self) {
        if self.selected.is_some() {
            self.selected = None;
            self.list.select(None);
            self.bump();
        }
    }

    /// Selects the first visible row.
    pub fn select_first(&mut self) -> bool {
        let first = self.flat.first().map(|row| row.path.clone());
        match first {
            Some(path) => self.select(path),
            None => {
                self.clear_selection();
                false
            }
        }
    }

    /// Selects the last visible row.
    pub fn select_last(&mut self) -> bool {
        let last = self.flat.last().map(|row| row.path.clone());
        match last {
            Some(path) => self.select(path),
            None => false,
        }
    }

    /// Moves the selection relative to the flattened list.
    pub fn select_relative(&mut self, delta: isize) -> bool {
        if self.flat.is_empty() {
            return false;
        }
        let current = self
            .flat
            .iter()
            .position(|row| self.selected.as_ref() == Some(&row.path));
        let next = match current {
            None => {
                if delta < 0 {
                    return false;
                }
                0
            }
            Some(index) => {
                let next = index as isize + delta;
                if next < 0 { 0 } else { next as usize }
            }
        };
        let next = next.min(self.flat.len() - 1);
        let path = self.flat[next].path.clone();
        self.select(path)
    }

    /// Rebuilds [`flat_rows`](Self::flat_rows) from `source` at `width`.
    pub fn rebuild_flat(&mut self, source: &impl TreeSource<Id>, width: u16) {
        self.source_revision = source.revision();
        self.layout_width = width;
        self.flat.clear();
        let filter_active = !self.filter.is_empty();
        let mut roots: Vec<TreePath<Id>> = Vec::new();
        let root_count = source.child_count(&[]);
        for index in 0..root_count {
            roots.push(vec![source.child_at(&[], index)]);
        }

        if filter_active {
            for path in roots {
                self.push_filtered(source, &path, 0, width);
            }
        } else {
            for path in roots {
                self.push_visible(source, &path, 0);
            }
        }

        self.sync_list_selection();
        self.ensure_selection_visible();
    }

    fn push_visible(&mut self, source: &impl TreeSource<Id>, path: &TreePath<Id>, depth: usize) {
        let is_branch = source.is_branch(path);
        let children_loaded = source.children_loaded(path);
        let is_open = is_branch && self.is_open(path);
        self.flat.push(FlatTreeRow {
            path: path.clone(),
            depth,
            is_branch,
            is_open,
            children_loaded,
            is_loading_placeholder: false,
        });

        if !is_open {
            return;
        }

        if is_branch && !children_loaded {
            self.flat.push(FlatTreeRow {
                path: path.clone(),
                depth: depth + 1,
                is_branch: false,
                is_open: false,
                children_loaded: true,
                is_loading_placeholder: true,
            });
            return;
        }

        let child_count = source.child_count(path);
        for index in 0..child_count {
            let child_path = {
                let mut child = path.clone();
                child.push(source.child_at(path, index));
                child
            };
            self.push_visible(source, &child_path, depth + 1);
        }
    }

    fn push_filtered(
        &mut self,
        source: &impl TreeSource<Id>,
        path: &TreePath<Id>,
        depth: usize,
        width: u16,
    ) {
        let label = source.label(path);
        let self_matches = self.node_matches(label);
        let mut child_paths: Vec<TreePath<Id>> = Vec::new();
        if source.children_loaded(path) || self_matches {
            let child_count = source.child_count(path);
            for index in 0..child_count {
                let mut child = path.clone();
                child.push(source.child_at(path, index));
                child_paths.push(child);
            }
        }

        let mut matching_children = Vec::new();
        for child in child_paths {
            if self.subtree_matches(source, &child, width) {
                matching_children.push(child);
            }
        }

        if !self_matches && matching_children.is_empty() {
            return;
        }

        let is_branch = source.is_branch(path) || !matching_children.is_empty();
        let is_open = is_branch;
        self.flat.push(FlatTreeRow {
            path: path.clone(),
            depth,
            is_branch,
            is_open,
            children_loaded: source.children_loaded(path),
            is_loading_placeholder: false,
        });

        for child in matching_children {
            self.push_filtered(source, &child, depth + 1, width);
        }
    }

    fn subtree_matches(
        &mut self,
        source: &impl TreeSource<Id>,
        path: &TreePath<Id>,
        width: u16,
    ) -> bool {
        if self.node_matches(source.label(path)) {
            return true;
        }
        if !source.children_loaded(path) {
            return false;
        }
        let _ = width;
        let child_count = source.child_count(path);
        for index in 0..child_count {
            let mut child = path.to_vec();
            child.push(source.child_at(path, index));
            if self.subtree_matches(source, &child, width) {
                return true;
            }
        }
        false
    }

    fn node_matches(&mut self, label: &str) -> bool {
        if self.filter.is_empty() {
            return true;
        }
        #[cfg(feature = "fuzzy")]
        {
            self.fuzzy.score(label).is_some()
        }
        #[cfg(not(feature = "fuzzy"))]
        {
            label
                .to_ascii_lowercase()
                .contains(&self.filter.to_ascii_lowercase())
        }
    }

    fn is_open(&self, path: &[Id]) -> bool {
        self.opened.contains(path)
    }

    fn sync_list_selection(&mut self) {
        let index = self
            .flat
            .iter()
            .position(|row| self.selected.as_ref() == Some(&row.path));
        self.list.select(index);
    }

    fn ensure_selection_visible(&mut self) {
        let index = self
            .flat
            .iter()
            .position(|row| self.selected.as_ref() == Some(&row.path));
        if let Some(index) = index {
            self.list.select(Some(index));
            let item_start = self.list.height_cache().item_start(index);
            let item_height = self.list.height_cache().item_height(index) as usize;
            let viewport = self.list.layout().viewport_height.max(1);
            let offset = self.list.scroll().offset();
            if item_start < offset {
                self.list.scroll_mut().scroll_to_offset(item_start);
            } else if item_start + item_height > offset + viewport as usize {
                let new_offset = item_start + item_height - viewport as usize;
                self.list.scroll_mut().scroll_to_offset(new_offset);
            }
        }
    }

    fn request_load(&mut self, path: TreePath<Id>) {
        self.pending_load = Some(path);
        self.bump();
    }
}

impl<Id> Component for TreeState<Id>
where
    Id: Clone + PartialEq + Eq + Hash,
{
    type Intent = TreeIntent<Id>;

    fn handle(&mut self, ev: &Event, keys: &Keymap) -> Outcome<Self::Intent> {
        let _ = keys;
        match ev {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Up => {
                    if self.select_relative(-1) {
                        self.ensure_selection_visible();
                        Outcome::Consumed
                    } else {
                        Outcome::Ignored
                    }
                }
                KeyCode::Down => {
                    if self.select_relative(1) {
                        self.ensure_selection_visible();
                        Outcome::Consumed
                    } else {
                        Outcome::Ignored
                    }
                }
                KeyCode::Left => {
                    if let Some(path) = self.selected.clone() {
                        if self.is_open(&path) {
                            self.close(&path);
                            Outcome::Consumed
                        } else if path.len() > 1 {
                            let parent = path[..path.len() - 1].to_vec();
                            self.select(parent);
                            Outcome::Consumed
                        } else {
                            Outcome::Ignored
                        }
                    } else {
                        Outcome::Ignored
                    }
                }
                KeyCode::Right => Outcome::Ignored,
                KeyCode::Enter | KeyCode::Char(' ') => Outcome::Ignored,
                _ => match self.list.handle(ev, keys) {
                    Outcome::Ignored => Outcome::Ignored,
                    Outcome::Consumed | Outcome::Intent(()) => Outcome::Consumed,
                },
            },
            _ => match self.list.handle(ev, keys) {
                Outcome::Ignored => Outcome::Ignored,
                Outcome::Consumed | Outcome::Intent(()) => Outcome::Consumed,
            },
        }
    }

    fn revision(&self) -> u64 {
        self.revision
            .max(Component::revision(&self.list))
            .max(self.source_revision)
    }
}

/// Extension methods that need a [`TreeSource`].
impl<Id> TreeState<Id>
where
    Id: Clone + PartialEq + Eq + Hash,
{
    /// Handles Right / Enter / Space with tree data for expand and lazy load.
    pub fn handle_with_source(
        &mut self,
        ev: &Event,
        keys: &Keymap,
        source: &impl TreeSource<Id>,
    ) -> Outcome<TreeIntent<Id>> {
        match ev {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Right => {
                    if let Some(path) = self.selected.clone() {
                        if source.is_branch(&path) {
                            if !source.children_loaded(&path) {
                                self.request_load(path.clone());
                                return Outcome::Intent(TreeIntent::LoadChildren(path));
                            }
                            self.open(path);
                            return Outcome::Consumed;
                        }
                    }
                    Outcome::Ignored
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(path) = self.selected.clone() {
                        if self.toggle(path.clone(), source) {
                            if let Some(load) = self.pending_load.clone() {
                                return Outcome::Intent(TreeIntent::LoadChildren(load));
                            }
                            return Outcome::Consumed;
                        }
                    }
                    Outcome::Ignored
                }
                _ => self.handle(ev, keys),
            },
            _ => self.handle(ev, keys),
        }
    }
}

/// Row height helper for [`TreeState`] when used with [`crate::ItemHeights`].
pub fn tree_row_height<Id: Clone + PartialEq + Eq + Hash>(
    state: &TreeState<Id>,
    row: &FlatTreeRow<Id>,
    label: &str,
    width: u16,
) -> u16 {
    if state.row_sizing() == TreeRowSizing::Fullscreen {
        return 1;
    }
    let indent = tree_indent_cols(row.depth) + 2;
    let cols = width.saturating_sub(indent).max(1) as usize;
    let lines = label.len().div_ceil(cols).max(1);
    lines as u16
}

/// Indent width in columns for a tree depth.
#[must_use]
pub fn tree_indent_cols(depth: usize) -> u16 {
    (depth * 2) as u16
}
