//! Hierarchical tree view built on [`VirtualList`] and [`ListState`](ratakit_core::ListState).

use ratakit_core::{
    FlatTreeRow, GlyphSet, ItemHeights, Revisioned, Theme, TreeRowSizing, TreeSource, TreeState,
};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::paragraph::Paragraph;

use crate::style::{fg, muted, selected_row_style};
use crate::virtual_list::{RowRenderer, VirtualList};

/// Tree view with expand/collapse and lazy-loaded children.
pub struct Tree<'a, Id, S> {
    source: &'a S,
    theme: &'a Theme,
    show_scrollbar: bool,
    _id: std::marker::PhantomData<Id>,
}

impl<'a, Id, S> Tree<'a, Id, S> {
    #[must_use]
    pub fn new(source: &'a S, theme: &'a Theme) -> Self {
        Self {
            source,
            theme,
            show_scrollbar: false,
            _id: std::marker::PhantomData,
        }
    }

    #[must_use]
    pub fn scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }
}

impl<Id, S> StatefulWidget for Tree<'_, Id, S>
where
    Id: Clone + PartialEq + Eq + std::hash::Hash,
    S: TreeSource<Id>,
{
    type State = TreeState<Id>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TreeState<Id>) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let content_width = if self.show_scrollbar && area.width > 1 {
            area.width - 1
        } else {
            area.width
        };

        state.rebuild_flat(self.source, content_width);
        let flat = state.flat_rows().to_vec();
        let selected = state.list().selected();
        let sizing = state.row_sizing();
        let revision = Revisioned::revision(state);
        let rows = TreeRows {
            source: self.source,
            flat: &flat,
            selected,
            sizing,
            revision,
            theme: self.theme,
        };
        let list = VirtualList::new(&rows, self.theme).scrollbar(self.show_scrollbar);
        list.render(area, buf, state.list_mut());
    }
}

struct TreeRows<'a, Id, S> {
    source: &'a S,
    flat: &'a [FlatTreeRow<Id>],
    selected: Option<usize>,
    sizing: TreeRowSizing,
    revision: u64,
    theme: &'a Theme,
}

impl<Id, S> ItemHeights for TreeRows<'_, Id, S>
where
    Id: Clone + PartialEq + Eq + std::hash::Hash,
    S: TreeSource<Id>,
{
    fn len(&self) -> usize {
        self.flat.len()
    }

    fn height(&self, index: usize, width: u16) -> u16 {
        match self.flat.get(index) {
            Some(row) => {
                let label = row_label(self.source, row);
                tree_row_height_for(self.sizing, row, label, width)
            }
            None => 1,
        }
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}

impl<Id, S> RowRenderer for TreeRows<'_, Id, S>
where
    Id: Clone + PartialEq + Eq + std::hash::Hash,
    S: TreeSource<Id>,
{
    fn render_row(&self, index: usize, area: Rect, buf: &mut Buffer, skip_top: u16) {
        let row = self.flat.get(index).expect("tree row in range");
        let label = row_label(self.source, row);
        let prefix = tree_prefix(self.theme, row);
        let line_text = format!("{prefix}{label}");
        let cols = area.width.max(1) as usize;
        let lines: Vec<Line> = line_text
            .chars()
            .collect::<Vec<_>>()
            .chunks(cols)
            .map(|chunk| {
                let text = chunk.iter().collect::<String>();
                let style = if row.is_loading_placeholder {
                    muted(self.theme)
                } else if self.selected == Some(index) {
                    selected_row_style(self.theme)
                } else {
                    fg(self.theme)
                };
                Line::from(Span::styled(text, style))
            })
            .collect();
        let start = skip_top.min(lines.len() as u16) as usize;
        let visible = lines.into_iter().skip(start).take(area.height as usize);
        Paragraph::new(visible.collect::<Vec<_>>()).render(area, buf);
    }
}

fn tree_row_height_for<Id>(
    sizing: TreeRowSizing,
    row: &FlatTreeRow<Id>,
    label: &str,
    width: u16,
) -> u16 {
    if sizing == TreeRowSizing::SingleLine {
        return 1;
    }
    let indent = ratakit_core::tree_indent_cols(row.depth) + 2;
    let cols = width.saturating_sub(indent).max(1) as usize;
    let lines = label.len().div_ceil(cols).max(1);
    lines as u16
}

fn row_label<'a, Id, S: TreeSource<Id>>(source: &'a S, row: &FlatTreeRow<Id>) -> &'a str {
    if row.is_loading_placeholder {
        "(loading…)"
    } else {
        source.label(&row.path)
    }
}

fn tree_prefix<Id>(theme: &Theme, row: &FlatTreeRow<Id>) -> String {
    let indent = "  ".repeat(row.depth);
    let glyph = if row.is_loading_placeholder || !row.is_branch {
        "  "
    } else {
        match (theme.glyphs, row.is_open) {
            (GlyphSet::Unicode, true) => "▼ ",
            (GlyphSet::Unicode, false) => "▶ ",
            (GlyphSet::Ascii, true) => "v ",
            (GlyphSet::Ascii, false) => "> ",
        }
    };
    format!("{indent}{glyph}")
}
