#![cfg(feature = "widgets")]

use ratakit_agent::model::{Effort, ModelCatalogEntry, ModelChoice};
use ratakit_agent::widgets::{ModelPicker, ModelPickerState};
use ratakit_core::{Component, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, Theme};
use ratakit_testing::render_stateful;

fn sample_catalog() -> Vec<ModelCatalogEntry> {
    vec![
        ModelCatalogEntry::new("OpenAI", "gpt-4.1")
            .with_detail("General reasoning model")
            .with_efforts(vec![Effort::Medium, Effort::High]),
        ModelCatalogEntry::new("OpenAI", "gpt-4.1-mini").with_efforts(vec![Effort::Low]),
        ModelCatalogEntry::new("Anthropic", "claude-sonnet-4")
            .with_detail("Balanced speed and quality")
            .with_efforts(vec![Effort::Low, Effort::Medium, Effort::High]),
    ]
}

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
    if style.contains("fg=rgb(137,180,250)") || style.contains("bg=rgb(137,180,250)") {
        return true;
    }
    if style.contains("fg=rgb(30,102,245)") || style.contains("bg=rgb(30,102,245)") {
        return true;
    }
    if style.contains("bold")
        && (style.contains("fg=rgb(108,112,134)") || style.contains("fg=rgb(108,111,133)"))
    {
        return true;
    }
    false
}

fn open_state() -> ModelPickerState {
    let mut state = ModelPickerState::new();
    state.set_catalog(sample_catalog());
    state.set_current(Some(ModelChoice {
        provider: "OpenAI".into(),
        model: "gpt-4.1".into(),
        effort: Some(Effort::Medium),
    }));
    state.open();
    state
}

#[test]
fn model_picker_fill_56x14_dark() {
    let theme = Theme::dark();
    let widget = ModelPicker::new(&theme);
    let mut state = open_state();
    let rendered = render_stateful(widget, 56, 14, &mut state);
    insta::assert_snapshot!(
        "model_picker_fill_56x14_dark",
        compact_picker_snapshot(rendered)
    );
}

#[test]
fn model_picker_query_56x14_dark() {
    let theme = Theme::dark();
    let widget = ModelPicker::new(&theme);
    let mut state = open_state();
    let keys = ratakit_core::Keymap::empty();
    for ch in "claude".chars() {
        let _ = state.handle(
            &Event::Key(KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
            }),
            &keys,
        );
    }
    let rendered = render_stateful(widget, 56, 14, &mut state);
    insta::assert_snapshot!(
        "model_picker_query_claude_56x14_dark",
        compact_picker_snapshot(rendered)
    );
}

#[test]
fn model_picker_effort_low_56x14_light() {
    let theme = Theme::light();
    let widget = ModelPicker::new(&theme);
    let mut state = open_state();
    let keys = ratakit_core::Keymap::empty();
    let _ = state.handle(
        &Event::Key(KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
        }),
        &keys,
    );
    let rendered = render_stateful(widget, 56, 14, &mut state);
    insta::assert_snapshot!(
        "model_picker_effort_low_56x14_light",
        compact_picker_snapshot(rendered)
    );
}
