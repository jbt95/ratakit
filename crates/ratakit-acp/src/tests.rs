use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::v1::{
    AgentCapabilities, ContentBlock, ContentChunk, InitializeRequest, InitializeResponse,
    NewSessionRequest, NewSessionResponse, Plan as AcpPlan, PlanEntry as AcpPlanEntry,
    PlanEntryPriority, PlanEntryStatus as AcpPlanEntryStatus, PromptRequest, PromptResponse,
    RequestPermissionOutcome, RequestPermissionRequest, SessionConfigOption,
    SessionConfigOptionCategory, SessionConfigSelectOption, SessionNotification, SessionUpdate,
    TextContent,
};
use agent_client_protocol::{Agent, Channel, on_receive_request};
use ratakit_agent::state::{
    AgentEvent, ContentPart, Effort, MessageId, ModelChoice, PermissionIntent, PermissionOptionId,
    PlanEntryStatus, StopReason, ToolContent, ToolStatus,
};

use crate::bridge::{
    AcpBridge, AgentCommand, BridgeEvent, permission_response, prompt_with_mentions,
};
use crate::mapping::{EventAccumulator, ModelRoutes, map_session_update, map_stop_reason};

fn empty_routes() -> Arc<Mutex<ModelRoutes>> {
    Arc::new(Mutex::new(ModelRoutes::default()))
}

#[test]
fn assistant_chunks_keep_their_message_id_and_finish_once() {
    let mut accumulator = EventAccumulator::default();
    let routes = empty_routes();
    let first = map_session_update(
        SessionUpdate::AgentMessageChunk(
            ContentChunk::new(ContentBlock::Text(TextContent::new("Hello")))
                .message_id(Some(agent_message_id("assistant-1"))),
        ),
        &mut accumulator,
        &routes,
    );
    let second = map_session_update(
        SessionUpdate::AgentMessageChunk(
            ContentChunk::new(ContentBlock::Text(TextContent::new(" world")))
                .message_id(Some(agent_message_id("assistant-1"))),
        ),
        &mut accumulator,
        &routes,
    );

    assert_eq!(
        first,
        vec![AgentEvent::AssistantDelta {
            id: MessageId::new("assistant-1"),
            text: "Hello".into(),
        }]
    );
    assert_eq!(
        second,
        vec![AgentEvent::AssistantDelta {
            id: MessageId::new("assistant-1"),
            text: " world".into(),
        }]
    );
    assert_eq!(
        accumulator.finish_turn(StopReason::EndTurn),
        vec![
            AgentEvent::MessageDone {
                id: MessageId::new("assistant-1"),
            },
            AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
            },
        ]
    );
}

#[test]
fn user_chunks_are_aggregated_before_agent_output() {
    let mut accumulator = EventAccumulator::default();
    let routes = empty_routes();
    let first = map_session_update(
        SessionUpdate::UserMessageChunk(
            ContentChunk::new(ContentBlock::Text(TextContent::new("question")))
                .message_id(Some(agent_message_id("user-1"))),
        ),
        &mut accumulator,
        &routes,
    );
    let second = map_session_update(
        SessionUpdate::AgentMessageChunk(
            ContentChunk::new(ContentBlock::Text(TextContent::new("answer")))
                .message_id(Some(agent_message_id("assistant-1"))),
        ),
        &mut accumulator,
        &routes,
    );

    assert!(first.is_empty());
    assert_eq!(
        second,
        vec![
            AgentEvent::UserMessage {
                id: MessageId::new("user-1"),
                content: vec![ContentPart::Text("question".into())],
            },
            AgentEvent::AssistantDelta {
                id: MessageId::new("assistant-1"),
                text: "answer".into(),
            },
        ]
    );
}

#[test]
fn tool_calls_map_status_locations_and_file_diffs() {
    let update: SessionUpdate = serde_json::from_value(serde_json::json!({
        "sessionUpdate": "tool_call",
        "toolCallId": "tool-1",
        "title": "Update README",
        "kind": "edit",
        "status": "in_progress",
        "locations": [{"path": "/repo/README.md"}],
        "content": [{
            "type": "diff",
            "path": "/repo/README.md",
            "oldText": "old text\n",
            "newText": "new text\n"
        }]
    }))
    .expect("tool call should deserialize");
    let mut accumulator = EventAccumulator::default();
    let events = map_session_update(update, &mut accumulator, &empty_routes());

    let [AgentEvent::ToolCallStarted(call)] = events.as_slice() else {
        panic!("expected a single tool call event, got {events:?}");
    };
    assert_eq!(call.status, ToolStatus::InProgress);
    assert_eq!(call.locations, vec![PathBuf::from("/repo/README.md")]);
    assert!(
        matches!(call.content.as_slice(), [ToolContent::Diff { path, .. }] if path == &PathBuf::from("/repo/README.md"))
    );
}

#[test]
fn plans_preserve_order_and_status() {
    let plan = AcpPlan::new(vec![
        AcpPlanEntry::new(
            "Inspect",
            PlanEntryPriority::High,
            AcpPlanEntryStatus::Completed,
        ),
        AcpPlanEntry::new(
            "Implement",
            PlanEntryPriority::Medium,
            AcpPlanEntryStatus::InProgress,
        ),
    ]);
    let update = SessionUpdate::Plan(plan);
    let mut accumulator = EventAccumulator::default();
    let events = map_session_update(update, &mut accumulator, &empty_routes());

    let [AgentEvent::PlanUpdated(plan)] = events.as_slice() else {
        panic!("expected a plan event, got {events:?}");
    };
    assert_eq!(plan.entries.len(), 2);
    assert_eq!(plan.entries[0].status, PlanEntryStatus::Completed);
    assert_eq!(plan.entries[1].status, PlanEntryStatus::InProgress);
}

#[test]
fn permission_options_preserve_ids_and_persistence() {
    let request = serde_json::from_value(serde_json::json!({
        "sessionId": "session-1",
        "toolCall": {
            "toolCallId": "tool-1",
            "title": "Run tests",
            "kind": "execute",
            "rawInput": {"command": "cargo test"}
        },
        "options": [
            {"optionId": "once", "name": "Allow once", "kind": "allow_once"},
            {"optionId": "always", "name": "Always allow", "kind": "allow_always"}
        ]
    }))
    .expect("permission request should deserialize");
    let mapped = crate::mapping::acp_permission_request(&request);

    assert_eq!(mapped.tool_call.id.as_str(), "tool-1");
    assert_eq!(mapped.tool_call.status, ToolStatus::Pending);
    assert!(
        matches!(&mapped.tool_call.content[0], ToolContent::Text(text) if text.contains("cargo test"))
    );
    assert!(!mapped.options[0].persistent);
    assert!(mapped.options[1].persistent);
}

#[test]
fn selected_permission_id_is_returned_to_the_agent() {
    let request: RequestPermissionRequest = serde_json::from_value(serde_json::json!({
        "sessionId": "session-1",
        "toolCall": {"toolCallId": "tool-1", "title": "Run tests"},
        "options": [
            {"optionId": "deny", "name": "Deny", "kind": "reject_once"},
            {"optionId": "allow", "name": "Allow", "kind": "allow_once"}
        ]
    }))
    .expect("permission request should deserialize");
    let response = permission_response(
        &request,
        PermissionIntent::Chose(PermissionOptionId::new("allow")),
    );

    assert!(matches!(
        response.outcome,
        RequestPermissionOutcome::Selected(selected)
            if selected.option_id.to_string() == "allow"
    ));
}

#[test]
fn stop_reasons_have_protocol_neutral_equivalents() {
    assert_eq!(
        map_stop_reason(agent_client_protocol::schema::v1::StopReason::MaxTokens),
        StopReason::MaxTokens
    );
    assert_eq!(
        map_stop_reason(agent_client_protocol::schema::v1::StopReason::Cancelled),
        StopReason::Cancelled
    );
}

#[test]
fn config_options_expose_model_and_effort_choices_with_routes() {
    let update: SessionUpdate = serde_json::from_value(serde_json::json!({
        "sessionUpdate": "config_option_update",
        "configOptions": [
            {
                "id": "model",
                "name": "Model",
                "category": "model",
                "type": "select",
                "currentValue": "model-a",
                "options": [{"value": "model-a", "name": "Model A"}]
            },
            {
                "id": "reasoning",
                "name": "Reasoning effort",
                "category": "thought_level",
                "type": "select",
                "currentValue": "medium",
                "options": [
                    {"value": "low", "name": "Low"},
                    {"value": "medium", "name": "Medium"}
                ]
            }
        ]
    }))
    .expect("configuration update should deserialize");
    let mut accumulator = EventAccumulator::default();
    let routes = empty_routes();
    let events = map_session_update(update, &mut accumulator, &routes);

    let [AgentEvent::ModelChoicesAvailable(choices)] = events.as_slice() else {
        panic!("expected model choices, got {events:?}");
    };
    assert!(choices.contains(&ModelChoice {
        provider: "model".into(),
        model: "model-a".into(),
        effort: None,
    }));
    assert!(choices.contains(&ModelChoice {
        provider: "model".into(),
        model: "model-a".into(),
        effort: Some(Effort::Low),
    }));
    let routes = routes.lock().expect("route lock should be healthy");
    assert!(routes.model_route(&choices[0]).is_some());
    assert!(routes.effort_route(Effort::Medium).is_some());
}

#[test]
fn usage_updates_map_context_fraction_and_usd_cost() {
    let update: SessionUpdate = serde_json::from_value(serde_json::json!({
        "sessionUpdate": "usage_update",
        "used": 40,
        "size": 100,
        "cost": {"amount": 0.25, "currency": "USD"}
    }))
    .expect("usage update should deserialize");
    let mut accumulator = EventAccumulator::default();
    let events = map_session_update(update, &mut accumulator, &empty_routes());

    let [AgentEvent::Usage(usage)] = events.as_slice() else {
        panic!("expected a usage event, got {events:?}");
    };
    assert_eq!(usage.context_used, Some(0.4));
    assert_eq!(usage.cost_usd, Some(0.25));
}

#[test]
fn mention_paths_are_sent_as_textual_context() {
    assert_eq!(
        prompt_with_mentions("Review this".into(), &[PathBuf::from("src/lib.rs")]),
        "Review this\n\nReferenced files:\n- @src/lib.rs\n"
    );
}

#[tokio::test]
async fn spawn_rejects_relative_working_directories_before_launching() {
    let result = AcpBridge::spawn(AgentCommand::new("not-launched"), Path::new("relative")).await;

    assert!(matches!(
        result,
        Err(crate::AcpError::RelativeWorkingDirectory(_))
    ));
}

#[tokio::test]
async fn bridge_runs_against_an_in_memory_acp_agent() {
    let (client_transport, agent_transport) = Channel::duplex();
    let fake_agent = tokio::spawn(async move {
        Agent
            .builder()
            .name("ratakit-test-agent")
            .on_receive_request(
                async move |request: InitializeRequest, responder, _connection| {
                    responder.respond(
                        InitializeResponse::new(request.protocol_version)
                            .agent_capabilities(AgentCapabilities::new()),
                    )
                },
                on_receive_request!(),
            )
            .on_receive_request(
                async move |_request: NewSessionRequest, responder, _connection| {
                    responder.respond(NewSessionResponse::new("session-test").config_options(
                        vec![SessionConfigOption::select(
                            "models",
                            "Models",
                            "model-a",
                            vec![SessionConfigSelectOption::new("model-a", "Model A")],
                        )
                        .category(SessionConfigOptionCategory::Model)],
                    ))
                },
                on_receive_request!(),
            )
            .on_receive_request(
                async move |request: PromptRequest, responder, connection| {
                    let update = SessionUpdate::AgentMessageChunk(
                        ContentChunk::new(ContentBlock::Text(TextContent::new("hello from fake")))
                            .message_id(Some(agent_message_id("assistant-fake"))),
                    );
                    connection
                        .send_notification(SessionNotification::new(request.session_id, update))?;
                    responder.respond(PromptResponse::new(
                        agent_client_protocol::schema::v1::StopReason::EndTurn,
                    ))
                },
                on_receive_request!(),
            )
            .connect_to(agent_transport)
            .await
    });

    let mut bridge = AcpBridge::connect(client_transport, PathBuf::from("/tmp"))
        .await
        .expect("bridge should initialize with the fake agent");
    let session = bridge
        .new_session()
        .await
        .expect("session creation should work");
    assert_eq!(session.as_str(), "session-test");
    let reason = bridge
        .handle()
        .prompt("hi", &[])
        .await
        .expect("fake prompt should complete");
    assert_eq!(reason, StopReason::EndTurn);

    let mut received = Vec::new();
    while let Ok(Some(event)) =
        tokio::time::timeout(std::time::Duration::from_millis(50), bridge.events().recv()).await
    {
        received.push(event);
        if received
            .iter()
            .any(|event| matches!(event, BridgeEvent::Agent(AgentEvent::TurnEnded { .. })))
        {
            break;
        }
    }
    assert!(received.iter().any(|event| matches!(
        event,
        BridgeEvent::Agent(AgentEvent::AssistantDelta { text, .. }) if text == "hello from fake"
    )));
    assert!(received.iter().any(|event| matches!(
        event,
        BridgeEvent::Agent(AgentEvent::ModelChoicesAvailable(choices))
            if choices.iter().any(|choice| choice.model == "model-a")
    )));
    assert!(received.iter().any(|event| matches!(
        event,
        BridgeEvent::Agent(AgentEvent::TurnEnded {
            reason: StopReason::EndTurn
        })
    )));

    drop(bridge);
    fake_agent.abort();
}

fn agent_message_id(value: &str) -> agent_client_protocol::schema::v1::MessageId {
    agent_client_protocol::schema::v1::MessageId::new(value)
}
