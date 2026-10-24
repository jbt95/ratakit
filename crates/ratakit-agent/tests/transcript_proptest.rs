use proptest::prelude::*;
use ratakit_agent::state::{
    AgentEvent, ContentPart, Level, MessageId, ToolCall, ToolCallId, ToolCallPatch, ToolKind,
    ToolStatus, TranscriptState,
};

fn arb_message_id() -> impl Strategy<Value = MessageId> {
    "\\PC{1,8}".prop_map(MessageId::new)
}

fn arb_tool_id() -> impl Strategy<Value = ToolCallId> {
    "\\PC{1,8}".prop_map(ToolCallId::new)
}

fn arb_event() -> impl Strategy<Value = AgentEvent> {
    prop_oneof![
        (arb_message_id(), "\\PC*").prop_map(|(id, text)| AgentEvent::UserMessage {
            id,
            content: vec![ContentPart::Text(text)],
        }),
        (arb_message_id(), "\\PC*").prop_map(|(id, text)| AgentEvent::AssistantDelta { id, text }),
        (arb_message_id(), "\\PC*").prop_map(|(id, text)| AgentEvent::ThinkingDelta { id, text }),
        arb_message_id().prop_map(|id| AgentEvent::MessageDone { id }),
        (
            arb_tool_id(),
            "\\PC{0,16}",
            prop_oneof![
                Just(ToolStatus::Pending),
                Just(ToolStatus::InProgress),
                Just(ToolStatus::Completed),
                Just(ToolStatus::Failed),
                Just(ToolStatus::Cancelled),
            ],
        )
            .prop_map(|(id, title, status)| {
                AgentEvent::ToolCallStarted(ToolCall {
                    id,
                    title,
                    kind: ToolKind::Other,
                    status,
                    locations: Vec::new(),
                    content: Vec::new(),
                })
            }),
        (
            arb_tool_id(),
            prop_oneof![
                Just(ToolStatus::Pending),
                Just(ToolStatus::InProgress),
                Just(ToolStatus::Completed),
            ],
        )
            .prop_map(|(id, status)| AgentEvent::ToolCallUpdated {
                id,
                patch: ToolCallPatch {
                    status: Some(status),
                    ..ToolCallPatch::default()
                },
            }),
        Just(AgentEvent::Notice {
            level: Level::Info,
            text: String::new(),
        }),
        "\\PC*".prop_map(AgentEvent::Error),
        Just(AgentEvent::TurnEnded {
            reason: ratakit_agent::state::StopReason::EndTurn,
        }),
    ]
}

proptest! {
    #[test]
    fn arbitrary_event_sequences_do_not_panic(events in prop::collection::vec(arb_event(), 0..64)) {
        let mut state = TranscriptState::new();
        for ev in events {
            state.apply(ev);
            let _ = state.revision();
            let _ = state.is_busy();
            let len = state.cells().len();
            if len > 0 {
                state.toggle_fold(ratakit_agent::state::CellId(len - 1));
            }
        }
    }
}
