// Transcript apply logic inspired by openai/codex `codex-rs/tui/src/history_cell/`
// (Apache-2.0). Modifications Copyright 2026 The ratakit Authors.

use std::collections::HashMap;

use super::cell::{
    AssistantCell, Cell, ErrorCell, NoticeCell, ThinkingCell, ToolCallCell, UserCell,
};
use super::event::AgentEvent;
use super::ids::{CellId, MessageId, ToolCallId};
use super::model::ModelChoice;
use super::plan::Plan;
use super::tool::{ToolCall, ToolCallPatch, ToolContent, ToolStatus};
use super::types::SlashCommand;
use super::usage::{ModeId, Usage};
use ratakit_core::Revisioned;

/// Transcript model: ordered cells plus session metadata updated via [`TranscriptState::apply`].
#[derive(Debug)]
pub struct TranscriptState {
    cells: Vec<Cell>,
    revision: u64,
    busy: bool,
    usage: Option<Usage>,
    mode: Option<ModeId>,
    slash_commands: Vec<SlashCommand>,
    model_choices: Vec<ModelChoice>,
    open_assistant: HashMap<MessageId, usize>,
    open_thinking: HashMap<MessageId, usize>,
    open_tools: HashMap<ToolCallId, usize>,
    plan_cell: Option<usize>,
}

impl TranscriptState {
    /// Empty transcript.
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            revision: 0,
            busy: false,
            usage: None,
            mode: None,
            slash_commands: Vec::new(),
            model_choices: Vec::new(),
            open_assistant: HashMap::new(),
            open_thinking: HashMap::new(),
            open_tools: HashMap::new(),
            plan_cell: None,
        }
    }

    /// Monotonic revision for layout caches.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Whether the agent is still working on the current turn.
    pub fn is_busy(&self) -> bool {
        self.busy
    }

    /// Latest usage snapshot from the agent.
    pub fn usage(&self) -> Option<&Usage> {
        self.usage.as_ref()
    }

    /// Current session mode, if reported.
    pub fn mode(&self) -> Option<&ModeId> {
        self.mode.as_ref()
    }

    /// Slash commands for the composer menu.
    pub fn slash_commands(&self) -> &[SlashCommand] {
        &self.slash_commands
    }

    /// Models the agent exposed for picking.
    pub fn model_choices(&self) -> &[ModelChoice] {
        &self.model_choices
    }

    /// All transcript cells in order.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Toggle fold state on thinking or tool-call cells.
    pub fn toggle_fold(&mut self, cell: CellId) {
        let idx = cell.index();
        if idx >= self.cells.len() {
            return;
        }
        match &mut self.cells[idx] {
            Cell::Thinking(t) => {
                t.folded = !t.folded;
                self.bump();
            }
            Cell::ToolCall(t) => {
                t.output_folded = !t.output_folded;
                self.bump();
            }
            Cell::User(_)
            | Cell::Assistant(_)
            | Cell::Plan(_)
            | Cell::Notice(_)
            | Cell::Error(_) => {}
        }
    }

    /// Apply a single agent event (sole mutator for transcript cells).
    pub fn apply(&mut self, ev: AgentEvent) {
        match ev {
            AgentEvent::UserMessage { id, content } => {
                self.busy = true;
                self.cells.push(Cell::User(UserCell { id, content }));
                self.bump();
            }
            AgentEvent::AssistantDelta { id, text } => {
                if text.is_empty() {
                    return;
                }
                self.busy = true;
                let idx = self.ensure_assistant_cell(id);
                if let Cell::Assistant(a) = &mut self.cells[idx] {
                    a.stream.push(&text);
                }
                self.bump();
            }
            AgentEvent::ThinkingDelta { id, text } => {
                if text.is_empty() {
                    return;
                }
                self.busy = true;
                let idx = self.ensure_thinking_cell(id);
                if let Cell::Thinking(t) = &mut self.cells[idx] {
                    t.stream.push(&text);
                }
                self.bump();
            }
            AgentEvent::MessageDone { id } => {
                self.finish_message(&id);
                self.bump();
            }
            AgentEvent::ToolCallStarted(call) => {
                self.busy = true;
                self.start_tool_call(call);
                self.bump();
            }
            AgentEvent::ToolCallUpdated { id, patch } => {
                self.busy = true;
                self.patch_tool_call(&id, patch);
                self.bump();
            }
            AgentEvent::PlanUpdated(plan) => {
                self.update_plan(plan);
                self.bump();
            }
            AgentEvent::Usage(usage) => {
                self.usage = Some(usage);
                self.bump();
            }
            AgentEvent::ModeChanged(mode) => {
                self.mode = Some(mode);
                self.bump();
            }
            AgentEvent::CommandsAvailable(commands) => {
                self.slash_commands = commands;
                self.bump();
            }
            AgentEvent::ModelChoicesAvailable(choices) => {
                self.model_choices = choices;
                self.bump();
            }
            AgentEvent::Notice { level, text } => {
                self.cells.push(Cell::Notice(NoticeCell { level, text }));
                self.bump();
            }
            AgentEvent::Error(text) => {
                self.cells.push(Cell::Error(ErrorCell { text }));
                self.bump();
            }
            AgentEvent::TurnEnded { reason: _ } => {
                self.busy = false;
                self.open_assistant.clear();
                self.open_thinking.clear();
                self.open_tools.clear();
                self.bump();
            }
        }
    }

    fn bump(&mut self) {
        self.revision += 1;
    }

    fn ensure_assistant_cell(&mut self, id: MessageId) -> usize {
        if let Some(&idx) = self.open_assistant.get(&id) {
            return idx;
        }
        let idx = self.cells.len();
        self.cells
            .push(Cell::Assistant(AssistantCell::new(id.clone())));
        self.open_assistant.insert(id, idx);
        idx
    }

    fn ensure_thinking_cell(&mut self, id: MessageId) -> usize {
        if let Some(&idx) = self.open_thinking.get(&id) {
            return idx;
        }
        let idx = self.cells.len();
        self.cells
            .push(Cell::Thinking(ThinkingCell::new(id.clone())));
        self.open_thinking.insert(id, idx);
        idx
    }

    fn finish_message(&mut self, id: &MessageId) {
        if let Some(idx) = self.open_assistant.remove(id) {
            if let Cell::Assistant(a) = &mut self.cells[idx] {
                a.stream.finish();
                a.done = true;
            }
        }
        if let Some(idx) = self.open_thinking.remove(id) {
            if let Cell::Thinking(t) = &mut self.cells[idx] {
                t.stream.finish();
                t.done = true;
                t.folded = true;
            }
        }
    }

    fn start_tool_call(&mut self, started: ToolCall) {
        let id = started.id.clone();
        if let Some(&idx) = self.open_tools.get(&id) {
            if let Cell::ToolCall(cell) = &mut self.cells[idx] {
                merge_tool_call(&mut cell.call, started);
                sync_tool_cell_from_call(cell);
            }
            return;
        }
        let idx = self.cells.len();
        let mut tool_cell = ToolCallCell::from_call(started);
        sync_tool_cell_from_call(&mut tool_cell);
        self.cells.push(Cell::ToolCall(tool_cell));
        self.open_tools.insert(id, idx);
    }

    fn patch_tool_call(&mut self, id: &ToolCallId, patch: ToolCallPatch) {
        let idx = if let Some(&idx) = self.open_tools.get(id) {
            idx
        } else {
            let placeholder = ToolCall {
                id: id.clone(),
                title: String::new(),
                kind: super::tool::ToolKind::Other,
                status: ToolStatus::Pending,
                locations: Vec::new(),
                content: Vec::new(),
            };
            let idx = self.cells.len();
            self.cells
                .push(Cell::ToolCall(ToolCallCell::from_call(placeholder)));
            self.open_tools.insert(id.clone(), idx);
            idx
        };

        if let Cell::ToolCall(cell) = &mut self.cells[idx] {
            patch.apply_to(&mut cell.call);
            if let Some(diff) = patch.diff {
                cell.diff = Some(diff);
            }
            sync_tool_cell_from_call(cell);
        }
    }

    fn update_plan(&mut self, plan: Plan) {
        if let Some(idx) = self.plan_cell {
            if idx < self.cells.len() {
                self.cells[idx] = Cell::Plan(plan);
                return;
            }
        }
        let idx = self.cells.len();
        self.cells.push(Cell::Plan(plan));
        self.plan_cell = Some(idx);
    }
}

impl Default for TranscriptState {
    fn default() -> Self {
        Self::new()
    }
}

impl Revisioned for TranscriptState {
    fn revision(&self) -> u64 {
        self.revision
    }
}

fn merge_tool_call(dst: &mut ToolCall, src: ToolCall) {
    dst.title = src.title;
    dst.kind = src.kind;
    dst.status = src.status;
    dst.locations = src.locations;
    if !src.content.is_empty() {
        dst.content = src.content;
    }
}

fn sync_tool_cell_from_call(cell: &mut ToolCallCell) {
    cell.input.clear();
    cell.output.clear();
    let mut diff = cell.diff.take();
    for chunk in &cell.call.content {
        match chunk {
            ToolContent::Text(text) => {
                if cell.call.status == ToolStatus::Pending
                    || cell.call.status == ToolStatus::InProgress
                {
                    if cell.input.is_empty() {
                        cell.input.clone_from(text);
                    } else {
                        cell.input.push('\n');
                        cell.input.push_str(text);
                    }
                } else if cell.output.is_empty() {
                    cell.output.clone_from(text);
                } else {
                    cell.output.push('\n');
                    cell.output.push_str(text);
                }
            }
            ToolContent::Diff { path: _, model } => {
                diff = Some(model.clone());
            }
        }
    }
    cell.diff = diff;
}
