#![cfg(feature = "fuzzy")]

use ratakit_core::{Component, PickerItem, PickerSecondaryAxis, PickerSection, PickerState, Theme};
use ratakit_testing::assert_stateful_widget_snapshot;
use ratakit_widgets::FuzzyPicker;

#[derive(Clone)]
struct FileItem {
    path: String,
    detail: Option<String>,
}

impl PickerItem for FileItem {
    fn label(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(&self.path)
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        self.detail.as_deref().map(std::borrow::Cow::Borrowed)
    }
}

fn open_picker() -> PickerState<FileItem> {
    let mut state = PickerState::new();
    state.set_sections(vec![
        PickerSection {
            title: "src".into(),
            items: vec![
                FileItem {
                    path: "src/main.rs".into(),
                    detail: Some("fn main() {}".into()),
                },
                FileItem {
                    path: "src/lib.rs".into(),
                    detail: Some("pub mod picker;".into()),
                },
            ],
        },
        PickerSection {
            title: "docs".into(),
            items: vec![FileItem {
                path: "docs/DISENO.md".into(),
                detail: Some("Design document".into()),
            }],
        },
    ]);
    state.open();
    state
}

#[test]
fn fuzzy_picker_fullscreen_80x20_dark() {
    let theme = Theme::dark();
    let widget = FuzzyPicker::new("Open file", &theme);
    let mut state = open_picker();
    assert_stateful_widget_snapshot!(
        "fuzzy_picker_fullscreen_80x20_dark",
        widget,
        &mut state,
        80,
        20
    );
}

#[test]
fn fuzzy_picker_query_highlight_80x20_dark() {
    let theme = Theme::dark();
    let widget = FuzzyPicker::new("Open file", &theme);
    let mut state = open_picker();
    state.list_state_mut().select(Some(2)); // first item row after typing handled via filter
    for ch in "lib".chars() {
        let _ = state.handle(
            &ratakit_core::Event::Key(ratakit_core::KeyEvent {
                code: ratakit_core::KeyCode::Char(ch),
                modifiers: ratakit_core::KeyModifiers::NONE,
                kind: ratakit_core::KeyEventKind::Press,
            }),
            &ratakit_core::Keymap::empty(),
        );
    }
    assert_stateful_widget_snapshot!(
        "fuzzy_picker_query_lib_80x20_dark",
        widget,
        &mut state,
        80,
        20
    );
}

#[test]
fn fuzzy_picker_preview_100x20_dark() {
    let theme = Theme::dark();
    let widget = FuzzyPicker::new("Open file", &theme).preview(true);
    let mut state = open_picker();
    assert_stateful_widget_snapshot!(
        "fuzzy_picker_preview_100x20_dark",
        widget,
        &mut state,
        100,
        20
    );
}

#[test]
fn fuzzy_picker_secondary_80x20_light() {
    let theme = Theme::light();
    #[derive(Clone)]
    struct Model {
        name: String,
        effort: String,
    }
    impl PickerItem for Model {
        fn label(&self) -> std::borrow::Cow<'_, str> {
            std::borrow::Cow::Borrowed(&self.name)
        }
        fn matches_secondary(&self, _idx: usize, label: &str) -> bool {
            self.effort == label
        }
    }
    let mut state = PickerState::new();
    state.set_sections(vec![PickerSection {
        title: "OpenAI".into(),
        items: vec![
            Model {
                name: "gpt-4.1".into(),
                effort: "medium".into(),
            },
            Model {
                name: "gpt-4.1-mini".into(),
                effort: "low".into(),
            },
        ],
    }]);
    state.set_secondary_axis(Some(PickerSecondaryAxis {
        label: "Effort".into(),
        options: vec!["low".into(), "medium".into()],
    }));
    state.open();
    let widget = FuzzyPicker::new("Choose model", &theme);
    assert_stateful_widget_snapshot!(
        "fuzzy_picker_secondary_80x20_light",
        widget,
        &mut state,
        80,
        20
    );
}
