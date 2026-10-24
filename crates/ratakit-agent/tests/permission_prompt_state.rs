#![cfg(feature = "widgets")]

use ratakit_agent::state::{
    PermissionIntent, PermissionOption, PermissionOptionId, PermissionPreview, PermissionRequest,
    ToolCall, ToolCallId, ToolContent, ToolKind, ToolStatus,
};
use ratakit_agent::widgets::PermissionPromptState;
use ratakit_core::{
    Component, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, Keymap, Outcome,
};

fn press(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
    })
}

fn sample_execute_request() -> PermissionRequest {
    PermissionRequest {
        tool_call: ToolCall {
            id: ToolCallId::new("t1"),
            title: "Run command".into(),
            kind: ToolKind::Execute,
            status: ToolStatus::Pending,
            locations: Vec::new(),
            content: vec![ToolContent::Text("cargo test -p ratakit-agent".into())],
        },
        options: acp_options(),
    }
}

fn acp_options() -> Vec<PermissionOption> {
    vec![
        PermissionOption {
            id: PermissionOptionId::new("allow_once"),
            label: "Allow once".into(),
            persistent: false,
        },
        PermissionOption {
            id: PermissionOptionId::new("allow_always"),
            label: "Allow always".into(),
            persistent: true,
        },
        PermissionOption {
            id: PermissionOptionId::new("reject_once"),
            label: "Reject once".into(),
            persistent: false,
        },
        PermissionOption {
            id: PermissionOptionId::new("reject_always"),
            label: "Reject always".into(),
            persistent: true,
        },
    ]
}

#[test]
fn preview_prefers_diff_over_text() {
    use ratakit_text::DiffModel;
    let model = DiffModel::from_texts("src/lib.rs", "old\n", "new\n");
    let request = PermissionRequest {
        tool_call: ToolCall {
            id: ToolCallId::new("edit"),
            title: "Edit file".into(),
            kind: ToolKind::Edit,
            status: ToolStatus::Pending,
            locations: vec!["src/lib.rs".into()],
            content: vec![
                ToolContent::Text("ignored".into()),
                ToolContent::Diff {
                    path: "src/lib.rs".into(),
                    model,
                },
            ],
        },
        options: acp_options(),
    };
    assert!(matches!(request.preview(), PermissionPreview::Diff(_)));
}

#[test]
fn digit_shortcut_selects_option() {
    let mut state = PermissionPromptState::new();
    let request = sample_execute_request();
    state.open_for(&request);
    let outcome = state.handle(&press(KeyCode::Char('2')), &Keymap::empty());
    assert_eq!(
        outcome,
        Outcome::Intent(PermissionIntent::Chose(PermissionOptionId::new(
            "allow_always"
        )))
    );
    assert!(!state.is_open());
}

#[test]
fn arrow_and_enter_confirm_selection() {
    let mut state = PermissionPromptState::new();
    state.open_for(&sample_execute_request());
    let _ = state.handle(&press(KeyCode::Right), &Keymap::empty());
    assert_eq!(state.selected_index(), 1);
    let _ = state.handle(&press(KeyCode::Left), &Keymap::empty());
    assert_eq!(state.selected_index(), 0);
    let outcome = state.handle(&press(KeyCode::Enter), &Keymap::empty());
    assert_eq!(
        outcome,
        Outcome::Intent(PermissionIntent::Chose(PermissionOptionId::new(
            "allow_once"
        )))
    );
}

#[test]
fn escape_dismisses() {
    let mut state = PermissionPromptState::new();
    state.open_for(&sample_execute_request());
    let outcome = state.handle(&press(KeyCode::Escape), &Keymap::empty());
    assert_eq!(outcome, Outcome::Intent(PermissionIntent::Dismissed));
    assert!(!state.is_open());
}
