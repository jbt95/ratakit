use ratakit_core::Theme;
use ratakit_testing::assert_stateful_widget_snapshot;
use ratakit_widgets::{
    ConfirmDialog, Dialog, DialogButton, DialogState, TabItem, Table, TableColumn, TableState,
    Tabs, TabsState,
};

fn open_dialog() -> DialogState {
    let mut state = DialogState::new();
    state.open();
    state
}

#[test]
fn dialog_confirm_40x10_dark() {
    let theme = Theme::dark();
    let buttons = [DialogButton::new("Cancel"), DialogButton::primary("Save")];
    let widget = Dialog::new(
        &theme,
        "Save changes?",
        "Your edits will be written to disk.",
        &buttons,
    );
    let mut state = open_dialog();
    state.select_button(1);
    assert_stateful_widget_snapshot!("dialog_confirm_40x10_dark", widget, &mut state, 40, 10);
}

#[test]
fn dialog_confirm_80x12_light() {
    let theme = Theme::light();
    let buttons = [DialogButton::new("No"), DialogButton::primary("Yes")];
    let widget = Dialog::new(&theme, "Quit?", "Unsaved work will be lost.", &buttons);
    let mut state = open_dialog();
    assert_stateful_widget_snapshot!("dialog_confirm_80x12_light", widget, &mut state, 80, 12);
}

#[test]
fn confirm_dialog_dangerous_80x10() {
    let theme = Theme::dark();
    let widget = ConfirmDialog::new(&theme, "Delete file?", "This cannot be undone.").dangerous();
    let mut state = open_dialog();
    assert_stateful_widget_snapshot!("confirm_dialog_dangerous_80x10", widget, &mut state, 80, 10);
}

#[test]
fn tabs_badges_80x3_dark() {
    let theme = Theme::dark();
    let items = [
        TabItem::new("Files"),
        TabItem::new("Search").with_badge(12),
        TabItem::new("Agent").with_active(true),
    ];
    let widget = Tabs::new(&theme, &items);
    let mut state = TabsState::new();
    state.set_tab_count(items.len());
    state.select(1);
    assert_stateful_widget_snapshot!("tabs_badges_80x3_dark", widget, &mut state, 80, 3);
}

#[test]
fn table_sorted_80x8_dark() {
    let theme = Theme::dark();
    let columns = [
        TableColumn::new("Name", vec!["alpha".into(), "beta".into(), "gamma".into()]),
        TableColumn::new("Size", vec!["1".into(), "22".into(), "333".into()]),
    ];
    let widget = Table::new(&theme, &columns);
    let mut state = TableState::new();
    state.set_dimensions(3, 2);
    state.select_row(Some(1));
    state.toggle_sort(0);
    assert_stateful_widget_snapshot!("table_sorted_80x8_dark", widget, &mut state, 80, 8);
}

#[test]
fn table_virtualized_40x6_light() {
    let theme = Theme::light();
    let rows: Vec<String> = (0..20).map(|i| format!("row-{i:02}")).collect();
    let columns = [
        TableColumn::new("#", rows),
        TableColumn::new("Status", vec!["ok".into(); 20]),
    ];
    let widget = Table::new(&theme, &columns);
    let mut state = TableState::new();
    state.set_dimensions(20, 2);
    state.select_row(Some(15));
    state.ensure_visible(4);
    assert_stateful_widget_snapshot!("table_virtualized_40x6_light", widget, &mut state, 40, 6);
}
