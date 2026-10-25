use ratakit_core::{CheckboxState, RadioGroupState, SelectState, Theme, ToggleState};
use ratakit_testing::{render_stateful, render_widget};
use ratakit_widgets::{
    Badge, Button, ButtonSize, Card, Checkbox, Orientation, Progress, RadioGroup, Select,
    SelectOption, Separator, Skeleton, Switch, Variant,
};
use ratatui::layout::Rect;

#[test]
fn button_and_badge_variants_render_labels() {
    let theme = Theme::dark();
    for (button, label) in [
        (Button::new(&theme, "Save"), "Save"),
        (
            Button::new(&theme, "Delete").variant(Variant::Destructive),
            "Delete",
        ),
        (
            Button::new(&theme, "Cancel")
                .size(ButtonSize::Small)
                .disabled(true),
            "Cancel",
        ),
    ] {
        assert!(render_widget(button, 20, 1).contains(label));
    }
    assert!(render_widget(Badge::new(&theme, "New"), 12, 1).contains("New"));
    assert!(
        render_widget(
            Badge::new(&theme, "Warning").variant(Variant::Outline),
            14,
            1
        )
        .contains("Warning")
    );
}

#[test]
fn checkbox_switch_and_radio_group_render_state() {
    let theme = Theme::dark();
    let mut checkbox = CheckboxState::new(true);
    let checkbox_text =
        render_stateful(Checkbox::new(&theme, "Notifications"), 30, 1, &mut checkbox);
    assert!(checkbox_text.contains("☑"));
    assert!(checkbox_text.contains("Notifications"));

    let mut switch = ToggleState::new(false);
    let switch_text = render_stateful(Switch::new(&theme, "Compact"), 24, 1, &mut switch);
    assert!(switch_text.contains("──○"));

    let mut radio = RadioGroupState::new();
    radio.set_option_count(3);
    radio.select(1);
    let radio_text = render_stateful(
        RadioGroup::new(&theme, &["System", "Dark", "Light"]),
        20,
        3,
        &mut radio,
    );
    assert!(radio_text.contains("Dark"));
    assert!(radio_text.contains("●"));
}

#[test]
fn select_renders_open_menu_and_committed_value() {
    let theme = Theme::dark();
    let options = [
        SelectOption::new("System"),
        SelectOption::new("Dark"),
        SelectOption::new("Light"),
    ];
    let mut state = SelectState::new();
    state.set_option_count(options.len());
    state.select(0);
    state.open();

    let rendered = render_stateful(Select::new(&theme, &options), 24, 4, &mut state);
    assert!(rendered.contains("System"));
    assert!(rendered.contains("Dark"));
    assert!(rendered.contains("Light"));
}

#[test]
fn card_separator_progress_and_skeleton_respect_their_areas() {
    let theme = Theme::dark();
    let card = Card::new(&theme).title("Settings");
    assert_eq!(card.inner(Rect::new(0, 0, 20, 6)), Rect::new(1, 1, 18, 4));
    assert!(render_widget(card, 20, 6).contains("Settings"));

    let separator = render_widget(
        Separator::new(&theme).orientation(Orientation::Vertical),
        1,
        4,
    );
    assert!(separator.contains('│'));

    let progress = render_widget(Progress::new(&theme, 0.72), 20, 1);
    assert!(progress.contains("72%"));
    let complete = render_widget(Progress::new(&theme, 2.0), 20, 1);
    assert!(complete.contains("100%"));

    let skeleton = render_widget(Skeleton::new(&theme), 8, 2);
    assert_eq!(skeleton.lines().take(2).count(), 2);
    assert!(skeleton.contains('░'));
}

#[test]
fn controls_use_ascii_markers_when_requested() {
    let mut theme = Theme::dark();
    theme.enable_screen_reader();
    let mut checkbox = CheckboxState::new(false);
    let rendered = render_stateful(Checkbox::new(&theme, "Updates"), 20, 1, &mut checkbox);
    assert!(rendered.contains("[ ]"));
    assert!(rendered.contains("Updates"));
}
