#![cfg(feature = "widgets")]

use std::path::PathBuf;

use ratakit_agent::state::{
    AgentEvent, ContentPart, Effort, MessageId, ModelCatalogEntry, ModelChoice, ModeId,
    PermissionOption, PermissionOptionId, PermissionRequest, Plan, PlanEntry, PlanEntryId,
    PlanEntryStatus, ToolCall, ToolCallId, ToolContent, ToolKind, ToolStatus, Usage,
};
use ratakit_agent::widgets::{AgentLayout, AgentLayoutState, agent_layout_focus};
use ratakit_core::Theme;
use ratakit_testing::render_stateful;
use ratakit_text::SyntectHighlighter;

struct EmptyFiles;

impl ratakit_agent::state::FileIndex for EmptyFiles {
    fn query(&self, _pattern: &str, _limit: usize) -> Vec<PathBuf> {
        Vec::new()
    }
}

fn sample_usage() -> Usage {
    Usage {
        input_tokens: 42_000,
        output_tokens: 900,
        cached_tokens: 4_000,
        context_used: Some(0.35),
        cost_usd: Some(0.02),
    }
}

fn seed_transcript(state: &mut AgentLayoutState) {
    state.apply_agent_event(AgentEvent::ModeChanged(ModeId::new("auto")));
    state.apply_agent_event(AgentEvent::Usage(sample_usage()));
    state.apply_agent_event(AgentEvent::UserMessage {
        id: MessageId::new("u1"),
        content: vec![ContentPart::Text("Explain ratakit AgentLayout".into())],
    });
    state.apply_agent_event(AgentEvent::AssistantDelta {
        id: MessageId::new("a1"),
        text: "AgentLayout composes **transcript**, composer, and overlays.".into(),
    });
    state.apply_agent_event(AgentEvent::MessageDone {
        id: MessageId::new("a1"),
    });
    state.apply_agent_event(AgentEvent::PlanUpdated(Plan {
        entries: vec![
            PlanEntry {
                id: PlanEntryId::new("1"),
                title: "Wire layout".into(),
                status: PlanEntryStatus::Completed,
            },
            PlanEntry {
                id: PlanEntryId::new("2"),
                title: "Add mock-agent".into(),
                status: PlanEntryStatus::InProgress,
            },
        ],
    }));
}

fn sample_catalog() -> Vec<ModelCatalogEntry> {
    vec![
        ModelCatalogEntry::new("Anthropic", "claude-sonnet-4")
            .with_detail("Balanced")
            .with_efforts(vec![Effort::Low, Effort::High]),
    ]
}

fn permission_request() -> PermissionRequest {
    PermissionRequest {
        tool_call: ToolCall {
            id: ToolCallId::new("run"),
            title: "Run tests".into(),
            kind: ToolKind::Execute,
            status: ToolStatus::Pending,
            locations: Vec::new(),
            content: vec![ToolContent::Text("cargo test -p ratakit-agent".into())],
        },
        options: vec![
            PermissionOption {
                id: PermissionOptionId::new("allow_once"),
                label: "Allow once".into(),
                persistent: false,
            },
            PermissionOption {
                id: PermissionOptionId::new("reject_once"),
                label: "Reject".into(),
                persistent: false,
            },
        ],
    }
}

fn compact_layout_snapshot(rendered: String) -> String {
    let lines: Vec<&str> = rendered.lines().collect();
    if lines.len() > 42 {
        let mut out = lines[..40].join("\n");
        out.push_str("\n…\n");
        if let Some(last) = lines.last() {
            out.push_str(last);
        }
        out
    } else {
        rendered
    }
}

#[test]
fn agent_layout_wide_80x18() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let highlighter = SyntectHighlighter;
    let files = EmptyFiles;
    let widget = AgentLayout::new(&theme, &highlighter, &files);
    let mut state = AgentLayoutState::new();
    seed_transcript(&mut state);
    state.set_model_catalog(sample_catalog());
    let rendered = render_stateful(widget, 80, 18, &mut state);
    insta::assert_snapshot!(
        "agent_layout_wide_80x18",
        compact_layout_snapshot(rendered)
    );
}

#[test]
fn agent_layout_narrow_48x14() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let highlighter = SyntectHighlighter;
    let files = EmptyFiles;
    let widget = AgentLayout::new(&theme, &highlighter, &files);
    let mut state = AgentLayoutState::new();
    seed_transcript(&mut state);
    let rendered = render_stateful(widget, 48, 14, &mut state);
    insta::assert_snapshot!(
        "agent_layout_narrow_48x14",
        compact_layout_snapshot(rendered)
    );
}

#[test]
fn agent_layout_permission_modal_56x16() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let highlighter = SyntectHighlighter;
    let files = EmptyFiles;
    let widget = AgentLayout::new(&theme, &highlighter, &files);
    let mut state = AgentLayoutState::new();
    seed_transcript(&mut state);
    state.present_permission(permission_request());
    let rendered = render_stateful(widget, 56, 16, &mut state);
    insta::assert_snapshot!(
        "agent_layout_permission_modal_56x16",
        compact_layout_snapshot(rendered)
    );
}

#[test]
fn agent_layout_model_picker_56x14() {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let highlighter = SyntectHighlighter;
    let files = EmptyFiles;
    let widget = AgentLayout::new(&theme, &highlighter, &files);
    let mut state = AgentLayoutState::new();
    seed_transcript(&mut state);
    state.set_model_catalog(sample_catalog());
    state.model_choice = Some(ModelChoice {
        provider: "Anthropic".into(),
        model: "claude-sonnet-4".into(),
        effort: Some(Effort::High),
    });
    state.model_picker.set_current(state.model_choice.clone());
    state
        .model_picker
        .open_with_trap(&mut state.focus, agent_layout_focus::MODEL_PICKER);
    let rendered = render_stateful(widget, 56, 14, &mut state);
    insta::assert_snapshot!(
        "agent_layout_model_picker_56x14",
        compact_layout_snapshot(rendered)
    );
}
