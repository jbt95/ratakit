//! ACP schema values mapped into ratakit's protocol-neutral agent state.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::v1::{
    AvailableCommandInput, ContentBlock, PermissionOptionKind, RequestPermissionRequest,
    SessionConfigKind, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOptions, SessionUpdate, StopReason as AcpStopReason,
    ToolCall as AcpToolCall, ToolCallContent, ToolCallStatus as AcpToolCallStatus,
    ToolCallUpdate as AcpToolCallUpdate, ToolCallUpdateFields, ToolKind as AcpToolKind,
};
use ratakit_agent::state::{
    AgentEvent, ContentPart, Effort, Level, MessageId, ModeId, ModelChoice, PermissionOption,
    PermissionOptionId, PermissionRequest, Plan, PlanEntry, PlanEntryId, PlanEntryStatus,
    SlashCommand, StopReason, ToolCall, ToolCallId, ToolCallPatch, ToolContent, ToolKind,
    ToolStatus, Usage,
};
use ratakit_text::DiffModel;

#[derive(Debug, Clone)]
pub(super) struct ModelRoute {
    pub(super) config_id: String,
    pub(super) value_id: String,
}

#[derive(Debug, Clone)]
pub(super) struct EffortRoute {
    pub(super) config_id: String,
    pub(super) value_id: String,
}

/// ACP configuration routes used by model and effort selection commands.
#[derive(Debug, Default)]
pub(super) struct ModelRoutes {
    models: HashMap<(String, String), ModelRoute>,
    efforts: HashMap<Effort, EffortRoute>,
}

impl ModelRoutes {
    pub(super) fn model_route(&self, choice: &ModelChoice) -> Option<ModelRoute> {
        self.models
            .get(&(choice.provider.clone(), choice.model.clone()))
            .cloned()
    }

    pub(super) fn effort_route(&self, effort: Effort) -> Option<EffortRoute> {
        self.efforts.get(&effort).cloned()
    }
}

#[derive(Debug)]
struct PendingUserMessage {
    id: MessageId,
    content: Vec<ContentPart>,
}

/// Correlates ACP's streamed chunks with ratakit's message lifecycle events.
#[derive(Debug, Default)]
pub(super) struct EventAccumulator {
    assistant_id: Option<MessageId>,
    thinking_id: Option<MessageId>,
    user_message: Option<PendingUserMessage>,
    next_message_id: u64,
}

impl EventAccumulator {
    fn message_id(&mut self, supplied: Option<impl ToString>, kind: &str) -> MessageId {
        if let Some(id) = supplied {
            return MessageId::new(id.to_string());
        }

        self.next_message_id = self.next_message_id.saturating_add(1);
        MessageId::new(format!("acp-{kind}-{}", self.next_message_id))
    }

    fn close_open_messages(&mut self, events: &mut Vec<AgentEvent>) {
        self.flush_user_message(events);
        if let Some(id) = self.assistant_id.take() {
            events.push(AgentEvent::MessageDone { id });
        }
        if let Some(id) = self.thinking_id.take() {
            events.push(AgentEvent::MessageDone { id });
        }
    }

    fn flush_user_message(&mut self, events: &mut Vec<AgentEvent>) {
        if let Some(message) = self.user_message.take() {
            events.push(AgentEvent::UserMessage {
                id: message.id,
                content: message.content,
            });
        }
    }

    /// Closes any outstanding message streams and marks the current turn complete.
    pub(super) fn finish_turn(&mut self, reason: StopReason) -> Vec<AgentEvent> {
        let mut events = Vec::with_capacity(3);
        self.close_open_messages(&mut events);
        events.push(AgentEvent::TurnEnded { reason });
        events
    }
}

// ACP updates share ordering state, so a single exhaustive dispatcher keeps
// message lifecycle transitions adjacent to the payload they close or start.
#[allow(clippy::too_many_lines)]
pub(super) fn map_session_update(
    update: SessionUpdate,
    accumulator: &mut EventAccumulator,
    model_routes: &Arc<Mutex<ModelRoutes>>,
) -> Vec<AgentEvent> {
    let mut events = Vec::new();
    match update {
        SessionUpdate::UserMessageChunk(chunk) => {
            if accumulator.assistant_id.is_some() || accumulator.thinking_id.is_some() {
                accumulator.close_open_messages(&mut events);
            }
            let id = accumulator.message_id(chunk.message_id, "user");
            let message = accumulator
                .user_message
                .get_or_insert_with(|| PendingUserMessage {
                    id: id.clone(),
                    content: Vec::new(),
                });
            if message.id != id {
                accumulator.flush_user_message(&mut events);
                accumulator.user_message = Some(PendingUserMessage {
                    id: id.clone(),
                    content: Vec::new(),
                });
            }
            if let Some(message) = accumulator.user_message.as_mut() {
                message.content.extend(map_content_part(chunk.content));
            }
        }
        SessionUpdate::AgentMessageChunk(chunk) => {
            accumulator.flush_user_message(&mut events);
            close_thinking(accumulator, &mut events);
            let id = accumulator.message_id(chunk.message_id, "assistant");
            close_changed_id(&mut accumulator.assistant_id, &id, &mut events);
            let text = content_text(chunk.content);
            if !text.is_empty() {
                events.push(AgentEvent::AssistantDelta {
                    id: id.clone(),
                    text,
                });
            }
            accumulator.assistant_id = Some(id);
        }
        SessionUpdate::AgentThoughtChunk(chunk) => {
            accumulator.flush_user_message(&mut events);
            close_assistant(accumulator, &mut events);
            let id = accumulator.message_id(chunk.message_id, "thinking");
            close_changed_id(&mut accumulator.thinking_id, &id, &mut events);
            let text = content_text(chunk.content);
            if !text.is_empty() {
                events.push(AgentEvent::ThinkingDelta {
                    id: id.clone(),
                    text,
                });
            }
            accumulator.thinking_id = Some(id);
        }
        SessionUpdate::ToolCall(call) => {
            accumulator.close_open_messages(&mut events);
            events.push(AgentEvent::ToolCallStarted(map_tool_call(call)));
        }
        SessionUpdate::ToolCallUpdate(update) => {
            accumulator.close_open_messages(&mut events);
            let id = ToolCallId::new(update.tool_call_id.to_string());
            let patch = map_tool_patch(update);
            events.push(AgentEvent::ToolCallUpdated { id, patch });
        }
        SessionUpdate::Plan(plan) => {
            events.push(AgentEvent::PlanUpdated(map_plan(plan)));
        }
        SessionUpdate::AvailableCommandsUpdate(update) => {
            let commands = update
                .available_commands
                .into_iter()
                .map(|command| {
                    let argument_hint = command.input.map(|input| match input {
                        AvailableCommandInput::Unstructured(input) => input.hint,
                        _ => String::new(),
                    });
                    SlashCommand {
                        name: command.name,
                        description: command.description,
                        argument_hint,
                    }
                })
                .collect();
            events.push(AgentEvent::CommandsAvailable(commands));
        }
        SessionUpdate::CurrentModeUpdate(update) => {
            events.push(AgentEvent::ModeChanged(ModeId::new(
                update.current_mode_id.to_string(),
            )));
        }
        SessionUpdate::ConfigOptionUpdate(update) => {
            if let Some(choices) = map_config_options(&update.config_options, model_routes) {
                events.push(AgentEvent::ModelChoicesAvailable(choices));
            }
        }
        SessionUpdate::UsageUpdate(update) => {
            let mut usage = Usage {
                context_used: context_fraction(update.used, update.size),
                ..Usage::default()
            };
            if let Some(cost) = update.cost
                && cost.currency.eq_ignore_ascii_case("USD")
            {
                usage.cost_usd = Some(cost.amount);
            }
            events.push(AgentEvent::Usage(usage));
        }
        SessionUpdate::Notice(notice) => {
            let level = match notice.severity {
                agent_client_protocol::schema::v1::NoticeSeverity::Warning => Level::Warning,
                agent_client_protocol::schema::v1::NoticeSeverity::Error => Level::Error,
                _ => Level::Info,
            };
            let text = notice
                .description
                .map_or(notice.title.clone(), |description| {
                    format!("{}: {description}", notice.title)
                });
            events.push(AgentEvent::Notice { level, text });
        }
        _ => {}
    }
    events
}

fn close_changed_id(
    current: &mut Option<MessageId>,
    next: &MessageId,
    events: &mut Vec<AgentEvent>,
) {
    if current.as_ref().is_some_and(|current| current != next)
        && let Some(id) = current.take()
    {
        events.push(AgentEvent::MessageDone { id });
    }
}

fn close_assistant(accumulator: &mut EventAccumulator, events: &mut Vec<AgentEvent>) {
    if let Some(id) = accumulator.assistant_id.take() {
        events.push(AgentEvent::MessageDone { id });
    }
}

fn close_thinking(accumulator: &mut EventAccumulator, events: &mut Vec<AgentEvent>) {
    if let Some(id) = accumulator.thinking_id.take() {
        events.push(AgentEvent::MessageDone { id });
    }
}

fn map_content_part(content: ContentBlock) -> Vec<ContentPart> {
    match content {
        ContentBlock::Text(text) => vec![ContentPart::Text(text.text)],
        ContentBlock::Image(image) => vec![ContentPart::Image {
            alt: image
                .annotations
                .and_then(|annotations| annotations.audience)
                .map_or_else(
                    || format!("{} image", image.mime_type),
                    |audience| format!("image for {audience:?}"),
                ),
        }],
        ContentBlock::Audio(_) => vec![ContentPart::Text("[audio content]".into())],
        ContentBlock::ResourceLink(resource) => {
            vec![ContentPart::Text(format!("[resource: {}]", resource.uri))]
        }
        ContentBlock::Resource(_) => {
            vec![ContentPart::Text("[embedded resource]".into())]
        }
        _ => Vec::new(),
    }
}

fn content_text(content: ContentBlock) -> String {
    match content {
        ContentBlock::Text(text) => text.text,
        ContentBlock::Image(image) => format!("[image: {}]", image.mime_type),
        ContentBlock::Audio(_) => "[audio content]".into(),
        ContentBlock::ResourceLink(resource) => format!("[resource: {}]", resource.uri),
        ContentBlock::Resource(_) => "[embedded resource]".into(),
        _ => String::new(),
    }
}

fn map_tool_call(call: AcpToolCall) -> ToolCall {
    let mut diff = None;
    let mut content = call
        .content
        .into_iter()
        .map(|item| map_tool_content(item, &mut diff))
        .collect::<Vec<_>>();
    append_raw_values(&mut content, call.raw_input, call.raw_output);

    ToolCall {
        id: ToolCallId::new(call.tool_call_id.to_string()),
        title: call.title,
        kind: map_tool_kind(call.kind),
        status: map_tool_status(call.status),
        locations: call
            .locations
            .into_iter()
            .map(|location| location.path)
            .collect(),
        content,
    }
}

fn map_tool_patch(update: AcpToolCallUpdate) -> ToolCallPatch {
    let ToolCallUpdateFields {
        kind,
        status,
        title,
        content,
        locations,
        raw_input,
        raw_output,
        ..
    } = update.fields;
    let mut diff = None;
    let mut content = content.map(|content| {
        content
            .into_iter()
            .map(|item| map_tool_content(item, &mut diff))
            .collect::<Vec<_>>()
    });
    let mut raw_content = Vec::new();
    append_raw_values(&mut raw_content, raw_input, raw_output);
    if !raw_content.is_empty() {
        content.get_or_insert_with(Vec::new).extend(raw_content);
    }

    ToolCallPatch {
        title,
        kind: kind.map(map_tool_kind),
        status: status.map(map_tool_status),
        locations: locations.map(|locations| {
            locations
                .into_iter()
                .map(|location| location.path)
                .collect()
        }),
        content,
        content_append: Vec::new(),
        diff,
    }
}

fn map_tool_content(content: ToolCallContent, diff: &mut Option<DiffModel>) -> ToolContent {
    match content {
        ToolCallContent::Diff(file_diff) => {
            let path = file_diff.path;
            let path_label = path.to_string_lossy().into_owned();
            let model = DiffModel::from_texts(
                &path_label,
                file_diff.old_text.as_deref().unwrap_or_default(),
                &file_diff.new_text,
            );
            *diff = Some(model.clone());
            ToolContent::Diff { path, model }
        }
        ToolCallContent::Content(content) => match content.content {
            ContentBlock::Text(text) => ToolContent::Text(text.text),
            other => ToolContent::Text(content_text(other)),
        },
        ToolCallContent::Terminal(terminal) => {
            ToolContent::Text(format!("[terminal {}]", terminal.terminal_id))
        }
        _ => ToolContent::Text("[unsupported tool content]".into()),
    }
}

fn value_to_text(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn append_raw_values(
    content: &mut Vec<ToolContent>,
    input: Option<serde_json::Value>,
    output: Option<serde_json::Value>,
) {
    if let Some(input) = input {
        content.push(ToolContent::Text(format!(
            "Input:\n{}",
            value_to_text(&input)
        )));
    }
    if let Some(output) = output {
        content.push(ToolContent::Text(format!(
            "Output:\n{}",
            value_to_text(&output)
        )));
    }
}

fn context_fraction(used: u64, size: u64) -> Option<f32> {
    if size == 0 {
        return None;
    }

    // The UI displays integer percentages, so normalize with integer arithmetic
    // before converting the bounded percentage to f32.
    let percent = u128::from(used.min(size)) * 100 / u128::from(size);
    let percent = u8::try_from(percent).unwrap_or(100);
    Some(f32::from(percent) / 100.0)
}

fn map_tool_kind(kind: AcpToolKind) -> ToolKind {
    match kind {
        AcpToolKind::Read => ToolKind::Read,
        AcpToolKind::Edit | AcpToolKind::Delete | AcpToolKind::Move => ToolKind::Edit,
        AcpToolKind::Execute => ToolKind::Execute,
        AcpToolKind::Search => ToolKind::Search,
        AcpToolKind::Fetch => ToolKind::Fetch,
        _ => ToolKind::Other,
    }
}

fn map_tool_status(status: AcpToolCallStatus) -> ToolStatus {
    match status {
        AcpToolCallStatus::InProgress => ToolStatus::InProgress,
        AcpToolCallStatus::Completed => ToolStatus::Completed,
        AcpToolCallStatus::Failed => ToolStatus::Failed,
        _ => ToolStatus::Pending,
    }
}

fn map_plan(plan: agent_client_protocol::schema::v1::Plan) -> Plan {
    Plan {
        entries: plan
            .entries
            .into_iter()
            .enumerate()
            .map(|(index, entry)| PlanEntry {
                id: PlanEntryId::new(index.to_string()),
                title: entry.content,
                status: match entry.status {
                    agent_client_protocol::schema::v1::PlanEntryStatus::Pending => {
                        PlanEntryStatus::Pending
                    }
                    agent_client_protocol::schema::v1::PlanEntryStatus::InProgress => {
                        PlanEntryStatus::InProgress
                    }
                    agent_client_protocol::schema::v1::PlanEntryStatus::Completed => {
                        PlanEntryStatus::Completed
                    }
                    _ => PlanEntryStatus::Pending,
                },
            })
            .collect(),
    }
}

pub(super) fn map_config_options(
    options: &[SessionConfigOption],
    routes: &Arc<Mutex<ModelRoutes>>,
) -> Option<Vec<ModelChoice>> {
    let mut mapped = ModelRoutes::default();
    let mut choices = Vec::new();
    let mut has_model_selector = false;

    let mut effort_values = Vec::new();
    for option in options {
        let values = match &option.kind {
            SessionConfigKind::Select(select) => match &select.options {
                SessionConfigSelectOptions::Ungrouped(options) => {
                    options.iter().collect::<Vec<_>>()
                }
                SessionConfigSelectOptions::Grouped(groups) => groups
                    .iter()
                    .flat_map(|group| group.options.iter())
                    .collect::<Vec<_>>(),
                _ => continue,
            },
            _ => continue,
        };
        match option.category.as_ref() {
            Some(SessionConfigOptionCategory::Model) => {
                has_model_selector = true;
                let provider = option.id.to_string();
                for value in values {
                    let model = value.value.to_string();
                    mapped.models.insert(
                        (provider.clone(), model.clone()),
                        ModelRoute {
                            config_id: provider.clone(),
                            value_id: model.clone(),
                        },
                    );
                    choices.push(ModelChoice {
                        provider: provider.clone(),
                        model,
                        effort: None,
                    });
                }
            }
            Some(SessionConfigOptionCategory::ThoughtLevel) => {
                for value in values {
                    if let Some(effort) = Effort::parse(&value.value.to_string())
                        .or_else(|| Effort::parse(&value.name.to_ascii_lowercase()))
                    {
                        mapped.efforts.insert(
                            effort,
                            EffortRoute {
                                config_id: option.id.to_string(),
                                value_id: value.value.to_string(),
                            },
                        );
                        effort_values.push(effort);
                    }
                }
            }
            _ => {}
        }
    }

    if !effort_values.is_empty() {
        let model_choices = choices.clone();
        for model in model_choices {
            for effort in &effort_values {
                choices.push(ModelChoice {
                    effort: Some(*effort),
                    ..model.clone()
                });
            }
        }
    }

    if let Ok(mut routes) = routes.lock() {
        *routes = mapped;
    }
    has_model_selector.then_some(choices)
}

pub(super) fn map_stop_reason(reason: AcpStopReason) -> StopReason {
    match reason {
        AcpStopReason::EndTurn | AcpStopReason::MaxTurnRequests => StopReason::EndTurn,
        AcpStopReason::MaxTokens => StopReason::MaxTokens,
        AcpStopReason::Cancelled => StopReason::Cancelled,
        AcpStopReason::Refusal => StopReason::Refusal,
        _ => StopReason::Error,
    }
}

pub(super) fn acp_permission_request(request: &RequestPermissionRequest) -> PermissionRequest {
    let update = request.tool_call.clone();
    let fields = update.fields;
    let mut diff = None;
    let mut content = fields
        .content
        .unwrap_or_default()
        .into_iter()
        .map(|content| map_tool_content(content, &mut diff))
        .collect::<Vec<_>>();
    append_raw_values(&mut content, fields.raw_input, fields.raw_output);
    let call = ToolCall {
        id: ToolCallId::new(update.tool_call_id.to_string()),
        title: fields.title.unwrap_or_else(|| "Permission request".into()),
        kind: fields.kind.map_or(ToolKind::Other, map_tool_kind),
        status: fields.status.map_or(ToolStatus::Pending, map_tool_status),
        locations: fields
            .locations
            .unwrap_or_default()
            .into_iter()
            .map(|location| location.path)
            .collect(),
        content,
    };
    let options = request
        .options
        .iter()
        .map(|option| PermissionOption {
            id: PermissionOptionId::new(option.option_id.to_string()),
            label: option.name.clone(),
            persistent: matches!(
                option.kind,
                PermissionOptionKind::AllowAlways | PermissionOptionKind::RejectAlways
            ),
        })
        .collect();
    PermissionRequest {
        tool_call: call,
        options,
    }
}
