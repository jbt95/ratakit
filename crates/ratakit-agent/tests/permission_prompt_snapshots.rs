#![cfg(feature = "widgets")]

use ratakit_agent::state::{
    PermissionOption, PermissionOptionId, PermissionRequest, ToolCall, ToolCallId, ToolContent,
    ToolKind, ToolStatus,
};
use ratakit_agent::widgets::{PermissionPrompt, PermissionPromptState};
use ratakit_core::Theme;
use ratakit_testing::render_stateful;
use ratakit_text::{DiffModel, SyntectHighlighter};

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

fn command_request() -> PermissionRequest {
    PermissionRequest {
        tool_call: ToolCall {
            id: ToolCallId::new("run"),
            title: "Run shell command".into(),
            kind: ToolKind::Execute,
            status: ToolStatus::Pending,
            locations: Vec::new(),
            content: vec![ToolContent::Text("cargo test -p ratakit-agent".into())],
        },
        options: acp_options(),
    }
}

fn diff_request() -> PermissionRequest {
    let model = DiffModel::from_texts(
        "src/main.rs",
        "fn main() {\n    println!(\"hi\");\n}\n",
        "fn main() {\n    println!(\"hello\");\n}\n",
    );
    PermissionRequest {
        tool_call: ToolCall {
            id: ToolCallId::new("edit"),
            title: "Apply patch".into(),
            kind: ToolKind::Edit,
            status: ToolStatus::Pending,
            locations: vec!["src/main.rs".into()],
            content: vec![ToolContent::Diff {
                path: "src/main.rs".into(),
                model,
            }],
        },
        options: acp_options(),
    }
}

fn compact_permission_snapshot(rendered: String) -> String {
    let Some((text, styles)) = rendered.split_once("--- styles ---\n") else {
        return rendered;
    };
    let compact_styles = styles
        .lines()
        .filter(|line| !line.is_empty())
        .filter(|line| interesting_style_line(line))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{text}--- styles ---\n{compact_styles}\n")
}

fn interesting_style_line(line: &str) -> bool {
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
    if style.contains("REVERSED") || style.contains("BOLD") {
        return true;
    }
    false
}

fn snapshot_prompt(name: &str, request: &PermissionRequest, width: u16, height: u16) {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let highlighter = SyntectHighlighter;
    let mut state = PermissionPromptState::new();
    state.open_for(request);
    let widget = PermissionPrompt::new(request, &theme, &highlighter);
    let rendered = render_stateful(widget, width, height, &mut state);
    insta::assert_snapshot!(name, compact_permission_snapshot(rendered));
}

#[test]
fn permission_prompt_command_snapshot() {
    let request = command_request();
    snapshot_prompt("permission_command_56x16_dark", &request, 56, 16);
}

#[test]
fn permission_prompt_diff_snapshot() {
    let request = diff_request();
    snapshot_prompt("permission_diff_56x18_dark", &request, 56, 18);
}

#[test]
fn permission_prompt_narrow_snapshot() {
    let request = command_request();
    snapshot_prompt("permission_command_32x14_dark", &request, 32, 14);
}
