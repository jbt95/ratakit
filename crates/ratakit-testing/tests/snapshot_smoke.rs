use ratakit_testing::{KeySeq, assert_widget_snapshot};
use ratatui::widgets::Paragraph;

#[test]
fn key_seq_parses_chords() {
    let seq = KeySeq::parse("ctrl+a h e y enter").expect("parse");
    assert_eq!(seq.events().len(), 5);
}

#[test]
fn paragraph_snapshot_20x3() {
    let widget = Paragraph::new("hello ratakit");
    assert_widget_snapshot!("paragraph_20x3", widget, 20, 3);
}
