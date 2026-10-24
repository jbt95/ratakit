#![cfg(feature = "widgets")]

use std::path::PathBuf;

use ratakit_agent::{Composer, ComposerState, ComposerWidgetState, FileIndex, SlashCommand};
use ratakit_core::{Event, KeyCode, KeyEvent, Keymap, Theme};
use ratakit_testing::render_stateful;

struct DemoFiles;

impl FileIndex for DemoFiles {
    fn query(&self, pattern: &str, limit: usize) -> Vec<PathBuf> {
        ["src/main.rs", "src/lib.rs", "docs/DISENO.md"]
            .into_iter()
            .filter(|p| p.contains(pattern))
            .take(limit)
            .map(PathBuf::from)
            .collect()
    }
}

fn render_composer(width: u16, height: u16, buffer_text: &str) -> String {
    let theme = Theme::dark();
    let files = DemoFiles;
    let mut composer = ComposerState::new();
    composer.set_slash_commands(vec![
        SlashCommand {
            name: "clear".into(),
            description: "Reset conversation".into(),
            argument_hint: None,
        },
        SlashCommand {
            name: "model".into(),
            description: "Pick a model".into(),
            argument_hint: None,
        },
    ]);
    composer.buffer.insert_str(buffer_text);
    composer.sync_completions(&files);

    let view = ComposerWidgetState::default();
    let widget = Composer::new(&theme, &files);
    render_stateful(widget, width, height, &mut (composer, view))
}

#[test]
fn composer_at_mention_40x8() {
    let rendered = render_composer(40, 8, "@sr");
    insta::assert_snapshot!("composer_at_mention_40x8", rendered);
}

#[test]
fn composer_slash_menu_40x6() {
    let rendered = render_composer(40, 6, "/mo");
    insta::assert_snapshot!("composer_slash_40x6", rendered);
}

#[test]
fn composer_queued_title_40x6() {
    let theme = Theme::dark();
    let files = DemoFiles;
    let mut composer = ComposerState::new();
    composer.set_agent_busy(true);
    composer.buffer.insert_str("queued item");
    let outcome = composer.handle(
        &Event::Key(KeyEvent::new(KeyCode::Enter)),
        &Keymap::empty(),
        &files,
    );
    assert!(matches!(outcome, ratakit_core::Outcome::Consumed));

    let view = ComposerWidgetState::default();
    let widget = Composer::new(&theme, &files);
    let rendered = render_stateful(widget, 40, 6, &mut (composer, view));
    insta::assert_snapshot!("composer_queued_40x6", rendered);
}
