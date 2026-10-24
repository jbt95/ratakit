use ratakit_core::{
    Event, KeyCode, KeyEvent, KeyEventKind, Keymap, Outcome, TreeIntent, TreeSource, TreeState,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Id {
    Root,
    Child,
    Grand,
    Lazy,
}

struct DemoTree {
    lazy_loaded: bool,
    revision: u64,
}

impl DemoTree {
    fn new() -> Self {
        Self {
            lazy_loaded: false,
            revision: 0,
        }
    }
}

impl TreeSource<Id> for DemoTree {
    fn child_count(&self, parent: &[Id]) -> usize {
        match parent {
            [] => 2,
            [Id::Root] => 1,
            [Id::Root, Id::Child] => 1,
            [Id::Lazy] if self.lazy_loaded => 1,
            [Id::Lazy] => 0,
            _ => 0,
        }
    }

    fn child_at(&self, parent: &[Id], index: usize) -> Id {
        match parent {
            [] => match index {
                0 => Id::Root,
                _ => Id::Lazy,
            },
            [Id::Root] => Id::Child,
            [Id::Root, Id::Child] => Id::Grand,
            [Id::Lazy] if self.lazy_loaded => Id::Child,
            _ => Id::Child,
        }
    }

    fn label(&self, path: &[Id]) -> &str {
        match path {
            [Id::Root] => "root",
            [Id::Root, Id::Child] => "child",
            [Id::Root, Id::Child, Id::Grand] => "grand",
            [Id::Lazy] => "lazy",
            [Id::Lazy, Id::Child] => "lazy-child",
            _ => "?",
        }
    }

    fn is_branch(&self, path: &[Id]) -> bool {
        matches!(path, [Id::Root] | [Id::Root, Id::Child] | [Id::Lazy])
    }

    fn children_loaded(&self, path: &[Id]) -> bool {
        match path {
            [Id::Lazy] => self.lazy_loaded,
            [Id::Root] | [Id::Root, Id::Child] => true,
            _ => false,
        }
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}

#[test]
fn tree_expand_and_flatten() {
    let source = DemoTree::new();
    let mut state = TreeState::new();
    state.open(vec![Id::Root]);
    state.open(vec![Id::Root, Id::Child]);
    state.rebuild_flat(&source, 40);
    assert_eq!(state.flat_rows().len(), 4);
    state.select(vec![Id::Root, Id::Child]);
    assert_eq!(state.selected(), Some(&[Id::Root, Id::Child][..]));
}

#[test]
fn tree_lazy_load_intent() {
    let source = DemoTree::new();
    let mut state = TreeState::new();
    state.rebuild_flat(&source, 40);
    state.select(vec![Id::Lazy]);
    let keys = Keymap::empty();
    let right = Event::Key(KeyEvent {
        code: KeyCode::Right,
        modifiers: Default::default(),
        kind: KeyEventKind::Press,
    });
    let outcome = state.handle_with_source(&right, &keys, &source);
    assert_eq!(
        outcome,
        Outcome::Intent(TreeIntent::LoadChildren(vec![Id::Lazy]))
    );
}

#[test]
fn tree_filter_substring_without_fuzzy_feature_still_compiles() {
    let source = DemoTree::new();
    let mut state = TreeState::new();
    state.set_filter("grand");
    state.open(vec![Id::Root]);
    state.open(vec![Id::Root, Id::Child]);
    state.rebuild_flat(&source, 40);
    let labels = state
        .flat_rows()
        .iter()
        .map(|row| source.label(&row.path))
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["root", "child", "grand"]);
}
