use ratakit_core::Theme;
use ratakit_testing::assert_stateful_widget_snapshot;
use ratakit_text::{DiffModel, SyntectHighlighter};
use ratakit_widgets::{DiffMode, DiffView, DiffViewState};

fn sample_model() -> DiffModel {
    let old = "fn greet(name: &str) {\n    println!(\"hi {}\", name);\n}\n";
    let new = "fn greet(name: &str) {\n    println!(\"hello {}\", name);\n}\n";
    DiffModel::from_texts("greet.rs", old, new)
}

#[test]
fn diff_view_unified_80x10() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let model = sample_model();
    let highlighter = SyntectHighlighter;
    let widget = DiffView::new(&model, &theme, &highlighter).mode(DiffMode::Unified);
    let mut state = DiffViewState::new();
    assert_stateful_widget_snapshot!("diff_unified_80x10", widget, &mut state, 80, 10);
}

#[test]
fn diff_view_unified_40x12() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let model = sample_model();
    let highlighter = SyntectHighlighter;
    let widget = DiffView::new(&model, &theme, &highlighter).mode(DiffMode::Unified);
    let mut state = DiffViewState::new();
    assert_stateful_widget_snapshot!("diff_unified_40x12", widget, &mut state, 40, 12);
}

#[test]
fn diff_view_split_80x10() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let model = sample_model();
    let highlighter = SyntectHighlighter;
    let widget = DiffView::new(&model, &theme, &highlighter).mode(DiffMode::SideBySide);
    let mut state = DiffViewState::new();
    assert_stateful_widget_snapshot!("diff_split_80x10", widget, &mut state, 80, 10);
}

#[test]
fn diff_view_split_40x12() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let model = sample_model();
    let highlighter = SyntectHighlighter;
    let widget = DiffView::new(&model, &theme, &highlighter).mode(DiffMode::SideBySide);
    let mut state = DiffViewState::new();
    assert_stateful_widget_snapshot!("diff_split_40x12", widget, &mut state, 40, 12);
}
