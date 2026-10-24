use ratakit_agent::state::CellId;
use ratakit_agent::state::{
    AgentEvent, Cell, ContentPart, Level, MessageId, Plan, PlanEntry, PlanEntryId, PlanEntryStatus,
    ToolCall, ToolCallId, ToolKind, ToolStatus, TranscriptState,
};
use ratakit_agent::widgets::Transcript;
use ratakit_core::Theme;
use ratakit_testing::assert_stateful_widget_snapshot;
use ratakit_testing::render_stateful_widget;
use ratakit_text::SyntectHighlighter;

fn mid(s: &str) -> MessageId {
    MessageId::new(s)
}

fn sample_transcript() -> TranscriptState {
    let mut t = TranscriptState::new();
    t.apply(AgentEvent::UserMessage {
        id: mid("u1"),
        content: vec![ContentPart::Text("Explain **bold** in one line.".into())],
    });
    t.apply(AgentEvent::AssistantDelta {
        id: mid("a1"),
        text: "Here is **markdown** streaming:\n\n- one\n- two".into(),
    });
    t.apply(AgentEvent::MessageDone { id: mid("a1") });
    t.apply(AgentEvent::Notice {
        level: Level::Info,
        text: "Context compacted".into(),
    });
    t.apply(AgentEvent::Error("turn failed: rate limit".into()));
    t
}

fn widget(theme: &Theme) -> Transcript<'_> {
    SyntectHighlighter::apply_theme(theme);
    Transcript::new(theme, &SyntectHighlighter)
}

#[test]
fn transcript_user_cell_40x4() {
    let theme = Theme::dark();
    let mut t = TranscriptState::new();
    t.apply(AgentEvent::UserMessage {
        id: mid("u"),
        content: vec![ContentPart::Text("Hello agent".into())],
    });
    assert_stateful_widget_snapshot!("transcript_user_40x4", widget(&theme), &mut t, 40, 4);
}

#[test]
fn transcript_assistant_done_40x8() {
    let theme = Theme::dark();
    let mut t = TranscriptState::new();
    t.apply(AgentEvent::AssistantDelta {
        id: mid("a"),
        text: "**Done** reply with a list:\n\n1. alpha\n2. beta".into(),
    });
    t.apply(AgentEvent::MessageDone { id: mid("a") });
    assert_stateful_widget_snapshot!(
        "transcript_assistant_done_40x8",
        widget(&theme),
        &mut t,
        40,
        8
    );
}

#[test]
fn transcript_assistant_streaming_40x6() {
    let theme = Theme::dark();
    let mut t = TranscriptState::new();
    t.apply(AgentEvent::AssistantDelta {
        id: mid("stream"),
        text: "Partial **bold** and\n\n```rust\nfn main() {".into(),
    });
    assert_stateful_widget_snapshot!(
        "transcript_assistant_streaming_40x6",
        widget(&theme),
        &mut t,
        40,
        6
    );
}

#[test]
fn transcript_thinking_folded_40x3() {
    let theme = Theme::dark();
    let mut t = TranscriptState::new();
    t.apply(AgentEvent::ThinkingDelta {
        id: mid("t"),
        text: "hidden reasoning line one\nline two".into(),
    });
    t.apply(AgentEvent::MessageDone { id: mid("t") });
    let Cell::Thinking(th) = &t.cells()[0] else {
        panic!("thinking cell");
    };
    assert!(th.folded);
    assert_stateful_widget_snapshot!(
        "transcript_thinking_folded_40x3",
        widget(&theme),
        &mut t,
        40,
        3
    );
}

#[test]
fn transcript_thinking_unfolded_40x6() {
    let theme = Theme::dark();
    let mut t = TranscriptState::new();
    t.apply(AgentEvent::ThinkingDelta {
        id: mid("t"),
        text: "Step **one**\n\nStep two".into(),
    });
    t.apply(AgentEvent::MessageDone { id: mid("t") });
    t.toggle_fold(CellId(0));
    assert_stateful_widget_snapshot!(
        "transcript_thinking_unfolded_40x6",
        widget(&theme),
        &mut t,
        40,
        6
    );
}

#[test]
fn transcript_notice_error_40x4() {
    let theme = Theme::dark();
    let mut t = TranscriptState::new();
    t.apply(AgentEvent::Notice {
        level: Level::Warning,
        text: "Low disk".into(),
    });
    t.apply(AgentEvent::Error("boom".into()));
    assert_stateful_widget_snapshot!(
        "transcript_notice_error_40x4",
        widget(&theme),
        &mut t,
        40,
        4
    );
}

#[test]
fn transcript_tool_plan_placeholder_40x5() {
    let theme = Theme::dark();
    let mut t = TranscriptState::new();
    t.apply(AgentEvent::ToolCallStarted(ToolCall {
        id: ToolCallId::new("tc1"),
        title: "read_file".into(),
        kind: ToolKind::Read,
        status: ToolStatus::InProgress,
        locations: Vec::new(),
        content: Vec::new(),
    }));
    t.apply(AgentEvent::PlanUpdated(Plan {
        entries: vec![
            PlanEntry {
                id: PlanEntryId::new("1"),
                title: "Scan repo".into(),
                status: PlanEntryStatus::Completed,
            },
            PlanEntry {
                id: PlanEntryId::new("2"),
                title: "Fix tests".into(),
                status: PlanEntryStatus::InProgress,
            },
        ],
    }));
    assert_stateful_widget_snapshot!(
        "transcript_tool_plan_placeholder_40x5",
        widget(&theme),
        &mut t,
        40,
        5
    );
}

#[test]
fn transcript_mixed_scrollbar_40x7() {
    let theme = Theme::dark();
    let mut t = sample_transcript();
    render_stateful_widget(
        Transcript::new(&theme, &SyntectHighlighter).scrollbar(true),
        &mut t,
        40,
        7,
    );
    let content_height = t.list().layout().content_height;
    t.list_mut()
        .scroll_mut()
        .scroll_by(-2, 7, content_height.max(1));
    let w = widget(&theme).scrollbar(true);
    assert_stateful_widget_snapshot!("transcript_mixed_scrollbar_40x7", w, &mut t, 40, 7);
}
