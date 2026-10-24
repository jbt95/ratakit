//! Headless smoke example: exercises core presets and renders one frame via TestBackend.

use ratakit_core::{Keymap, KeymapPreset, Theme};
use ratakit_testing::render_to_string;
use ratatui::widgets::Paragraph;
use ratatui_widgets::block::Block;
use ratatui_widgets::borders::Borders;

fn main() {
    let _theme = Theme::dark();
    let _keys = Keymap::from_preset(KeymapPreset::Default);

    let widget = Paragraph::new("ratakit milestone 0").block(
        Block::default()
            .title("hello-headless")
            .borders(Borders::ALL),
    );

    let snapshot = render_to_string(widget, 32, 5);
    assert!(snapshot.contains("ratakit"));
    println!("ok: rendered {} bytes", snapshot.len());
}
