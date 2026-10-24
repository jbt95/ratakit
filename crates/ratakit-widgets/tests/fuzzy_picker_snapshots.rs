#![cfg(feature = "fuzzy")]

use ratakit_core::{Component, PickerItem, PickerSecondaryAxis, PickerSection, PickerState, Theme};
use ratakit_testing::render_stateful;
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

/// Plain-text grid plus non-default styles (skip panel fill and default fg-on-panel).
fn compact_picker_snapshot(rendered: String) -> String {
    let Some((text, styles)) = rendered.split_once("--- styles ---\n") else {
        return rendered;
    };
    let compact_styles = styles
        .lines()
        .filter(|line| !line.is_empty())
        .filter(|line| interesting_picker_style_line(line))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{text}--- styles ---\n{compact_styles}\n")
}

fn interesting_picker_style_line(line: &str) -> bool {
    let Some(style) = line.split(": ").nth(1) else {
        return false;
    };
    if style.contains("italic") || style.contains("underline") {
        return true;
    }
    // Accent (prompt, fuzzy match, selection cursor).
    if style.contains("fg=rgb(137,180,250)") || style.contains("bg=rgb(137,180,250)") {
        return true;
    }
    if style.contains("fg=rgb(30,102,245)") || style.contains("bg=rgb(30,102,245)") {
        return true;
    }
    // Muted section headers and secondary-axis chips (bold only).
    if style.contains("bold")
        && (style.contains("fg=rgb(108,112,134)") || style.contains("fg=rgb(108,111,133)"))
    {
        return true;
    }
    false
}

fn assert_picker_snapshot<W: ratatui::widgets::StatefulWidget>(
    name: &str,
    widget: W,
    state: &mut W::State,
    width: u16,
    height: u16,
) {
    let rendered = render_stateful(widget, width, height, state);
    insta::assert_snapshot!(name, compact_picker_snapshot(rendered));
}

#[test]
fn fuzzy_picker_fill_56x14_dark() {
    let theme = Theme::dark();
    let widget = FuzzyPicker::new("Open file", &theme);
    let mut state = open_picker();
    assert_picker_snapshot("fuzzy_picker_fill_56x14_dark", widget, &mut state, 56, 14);
}

#[test]
fn fuzzy_picker_query_highlight_56x14_dark() {
    let theme = Theme::dark();
    let widget = FuzzyPicker::new("Open file", &theme);
    let mut state = open_picker();
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
    assert_picker_snapshot(
        "fuzzy_picker_query_lib_56x14_dark",
        widget,
        &mut state,
        56,
        14,
    );
}

#[test]
fn fuzzy_picker_preview_72x14_dark() {
    let theme = Theme::dark();
    let widget = FuzzyPicker::new("Open file", &theme).preview(true);
    let mut state = open_picker();
    assert_picker_snapshot(
        "fuzzy_picker_preview_72x14_dark",
        widget,
        &mut state,
        72,
        14,
    );
}

#[test]
fn fuzzy_picker_secondary_56x14_light() {
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
    assert_picker_snapshot(
        "fuzzy_picker_secondary_56x14_light",
        widget,
        &mut state,
        56,
        14,
    );
}
