use ratakit_core::{
    Component, Event, KeyCode, KeyEvent, KeyEventKind, Keymap, Theme, TreeSource, TreeState,
};
use ratakit_testing::assert_stateful_widget_snapshot;
use ratakit_widgets::{ScrollView, ScrollViewContent, ScrollViewMode, Tree};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;
use ratatui_widgets::paragraph::Paragraph;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Node {
    Src,
    MainRs,
    LibRs,
}

struct FileTree;

impl TreeSource<Node> for FileTree {
    fn child_count(&self, parent: &[Node]) -> usize {
        match parent {
            [] => 1,
            [Node::Src] => 2,
            _ => 0,
        }
    }

    fn child_at(&self, parent: &[Node], index: usize) -> Node {
        match parent {
            [] => Node::Src,
            [Node::Src] => match index {
                0 => Node::MainRs,
                _ => Node::LibRs,
            },
            _ => Node::MainRs,
        }
    }

    fn label(&self, path: &[Node]) -> &str {
        match path.last() {
            Some(Node::Src) => "src",
            Some(Node::MainRs) => "main.rs",
            Some(Node::LibRs) => "lib.rs",
            None => "",
        }
    }

    fn is_branch(&self, path: &[Node]) -> bool {
        matches!(path, [] | [Node::Src])
    }

    fn children_loaded(&self, _path: &[Node]) -> bool {
        true
    }
}

#[test]
fn tree_expanded_40x6() {
    let theme = Theme::dark();
    let source = FileTree;
    let mut state = TreeState::new();
    state.open(vec![Node::Src]);
    state.select(vec![Node::Src, Node::MainRs]);
    let widget = Tree::new(&source, &theme);
    assert_stateful_widget_snapshot!("tree_expanded_40x6", widget, &mut state, 40, 6);
}

#[test]
fn tree_with_scrollbar_30x4() {
    let theme = Theme::dark();
    let source = FileTree;
    let mut state = TreeState::new();
    state.open(vec![Node::Src]);
    let widget = Tree::new(&source, &theme).scrollbar(true);
    assert_stateful_widget_snapshot!("tree_scrollbar_30x4", widget, &mut state, 30, 4);
}

struct LinesContent {
    lines: Vec<String>,
}

impl ScrollViewContent for LinesContent {
    fn content_height(&self, _width: u16) -> usize {
        self.lines.len()
    }

    fn render(&self, area: Rect, buf: &mut Buffer, y_offset: usize) {
        for row in 0..area.height as usize {
            let line_index = y_offset + row;
            if line_index >= self.lines.len() {
                break;
            }
            let row_area = Rect {
                x: area.x,
                y: area.y + row as u16,
                width: area.width,
                height: 1,
            };
            Paragraph::new(Line::from(Span::raw(self.lines[line_index].clone())))
                .style(Style::default())
                .render(row_area, buf);
        }
    }
}

#[test]
fn scroll_view_scrolled_40x5() {
    let theme = Theme::dark();
    let content = LinesContent {
        lines: (1..=12).map(|n| format!("line {n:02}")).collect(),
    };
    let widget = ScrollView::new(&content, &theme)
        .mode(ScrollViewMode::Fit)
        .scrollbar(true);
    let mut state = ratakit_core::ScrollState::new();
    state.sync_layout(5, content.content_height(40));
    state.scroll_by(3, 5, content.content_height(40));
    assert_stateful_widget_snapshot!("scroll_view_40x5", widget, &mut state, 40, 5);
}

#[test]
fn tree_keyboard_moves_selection() {
    let source = FileTree;
    let mut state = TreeState::new();
    state.open(vec![Node::Src]);
    state.rebuild_flat(&source, 40);
    state.select_first();
    let keys = Keymap::empty();
    let down = Event::Key(KeyEvent {
        code: KeyCode::Down,
        modifiers: Default::default(),
        kind: KeyEventKind::Press,
    });
    state.handle(&down, &keys);
    assert_eq!(state.selected(), Some(&[Node::Src, Node::MainRs][..]));
}
