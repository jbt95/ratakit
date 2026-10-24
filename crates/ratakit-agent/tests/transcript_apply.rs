use ratakit_agent::state::{
    AgentEvent, Cell, ContentPart, MessageId, Plan, PlanEntry, PlanEntryId, PlanEntryStatus,
    ToolCall, ToolCallId, ToolCallPatch, ToolContent, ToolKind, ToolStatus, TranscriptState,
};
use ratakit_text::DiffModel;

fn mid(s: &str) -> MessageId {
    MessageId::new(s)
}

fn tid(s: &str) -> ToolCallId {
    ToolCallId::new(s)
}

#[test]
fn assistant_streaming_deltas_accumulate() {
    let mut t = TranscriptState::new();
    let id = mid("a1");
    t.apply(AgentEvent::AssistantDelta {
        id: id.clone(),
        text: "Hello ".into(),
    });
    t.apply(AgentEvent::AssistantDelta {
        id: id.clone(),
        text: "world".into(),
    });
    t.apply(AgentEvent::MessageDone { id });
    assert_eq!(t.cells().len(), 1);
    let Cell::Assistant(a) = &t.cells()[0] else {
        panic!("expected assistant cell");
    };
    assert!(a.done);
    assert!(a.stream.source().contains("Hello world"));
}

#[test]
fn thinking_folds_when_done() {
    let mut t = TranscriptState::new();
    let id = mid("think");
    t.apply(AgentEvent::ThinkingDelta {
        id: id.clone(),
        text: "hmm".into(),
    });
    t.apply(AgentEvent::MessageDone { id });
    let Cell::Thinking(th) = &t.cells()[0] else {
        panic!("expected thinking");
    };
    assert!(th.done);
    assert!(th.folded);
}

#[test]
fn toggle_fold_thinking_and_tool() {
    let mut t = TranscriptState::new();
    let id = mid("t");
    t.apply(AgentEvent::ThinkingDelta {
        id,
        text: "x".into(),
    });
    t.apply(AgentEvent::MessageDone { id: mid("t") });
    let Cell::Thinking(th) = &t.cells()[0] else {
        panic!();
    };
    assert!(th.folded);
    t.toggle_fold(ratakit_agent::state::CellId(0));
    let Cell::Thinking(th) = &t.cells()[0] else {
        panic!();
    };
    assert!(!th.folded);

    t.apply(AgentEvent::ToolCallStarted(ToolCall {
        id: tid("tool"),
        title: "run".into(),
        kind: ToolKind::Execute,
        status: ToolStatus::InProgress,
        locations: Vec::new(),
        content: vec![ToolContent::Text("ls".into())],
    }));
    t.toggle_fold(ratakit_agent::state::CellId(1));
    let Cell::ToolCall(tc) = &t.cells()[1] else {
        panic!();
    };
    assert!(tc.output_folded);
}

#[test]
fn tool_lifecycle_and_out_of_order_patch() {
    let mut t = TranscriptState::new();
    let id = tid("tc1");
    t.apply(AgentEvent::ToolCallUpdated {
        id: id.clone(),
        patch: ToolCallPatch {
            status: Some(ToolStatus::InProgress),
            title: Some("edit file".into()),
            ..ToolCallPatch::default()
        },
    });
    assert_eq!(t.cells().len(), 1);
    t.apply(AgentEvent::ToolCallStarted(ToolCall {
        id: id.clone(),
        title: "edit file".into(),
        kind: ToolKind::Edit,
        status: ToolStatus::InProgress,
        locations: Vec::new(),
        content: vec![ToolContent::Text("patch body".into())],
    }));
    let Cell::ToolCall(cell) = &t.cells()[0] else {
        panic!();
    };
    assert_eq!(cell.status(), ToolStatus::InProgress);
    assert!(cell.input.contains("patch"));

    let diff = DiffModel::from_texts("a.rs", "old\n", "new\n");
    t.apply(AgentEvent::ToolCallUpdated {
        id,
        patch: ToolCallPatch {
            status: Some(ToolStatus::Completed),
            diff: Some(diff),
            content_append: vec![ToolContent::Text("done".into())],
            ..ToolCallPatch::default()
        },
    });
    let Cell::ToolCall(cell) = &t.cells()[0] else {
        panic!();
    };
    assert_eq!(cell.status(), ToolStatus::Completed);
    assert!(cell.diff.is_some());
}

#[test]
fn turn_end_clears_busy() {
    let mut t = TranscriptState::new();
    t.apply(AgentEvent::UserMessage {
        id: mid("u"),
        content: vec![ContentPart::Text("hi".into())],
    });
    assert!(t.is_busy());
    t.apply(AgentEvent::TurnEnded {
        reason: ratakit_agent::state::StopReason::EndTurn,
    });
    assert!(!t.is_busy());
}

#[test]
fn plan_cell_updates_in_place() {
    let mut t = TranscriptState::new();
    let plan1 = Plan {
        entries: vec![PlanEntry {
            id: PlanEntryId::new("1"),
            title: "first".into(),
            status: PlanEntryStatus::Pending,
        }],
    };
    t.apply(AgentEvent::PlanUpdated(plan1));
    t.apply(AgentEvent::PlanUpdated(Plan {
        entries: vec![PlanEntry {
            id: PlanEntryId::new("1"),
            title: "done".into(),
            status: PlanEntryStatus::Completed,
        }],
    }));
    assert_eq!(t.cells().len(), 1);
    let Cell::Plan(p) = &t.cells()[0] else {
        panic!();
    };
    assert_eq!(p.entries[0].status, PlanEntryStatus::Completed);
}
