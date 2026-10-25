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
use ratakit_core::{FuzzyFilter, ListState};
use unicode_segmentation::UnicodeSegmentation;

#[cfg(feature = "widgets")]
use crate::tool_call_aux::ToolCallCardAux;

/// Transcript model: ordered cells plus session metadata updated via [`TranscriptState::apply`].
#[derive(Debug)]
pub struct TranscriptState {
    cells: Vec<Cell>,
    list: ListState,
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
    pending_search_scroll: Option<usize>,
    active_search_result: Option<TranscriptSearchResult>,
    #[cfg(feature = "widgets")]
    tool_call_aux: Vec<ToolCallCardAux>,
}

/// A fuzzy match in a transcript cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptSearchResult {
    /// Index of the matching cell in [`TranscriptState::cells`].
    pub cell_index: usize,
    /// Nucleo fuzzy score; higher scores are more relevant.
    pub score: u32,
    /// Short excerpt around the match, suitable for a search-results list.
    pub preview: String,
    /// Grapheme indices in the searchable cell text that contributed to the match.
    pub matched_indices: Vec<u32>,
    transcript_revision: u64,
}

/// Cyclic navigation through a transcript search result set.
#[derive(Debug, Clone, Default)]
pub struct TranscriptSearchNavigator {
    results: Vec<TranscriptSearchResult>,
    selected: Option<usize>,
    transcript_revision: u64,
}

impl TranscriptSearchNavigator {
    /// Search results in relevance order.
    pub fn results(&self) -> &[TranscriptSearchResult] {
        &self.results
    }

    /// Current result index in [`Self::results`].
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    /// Current result, if navigation has selected one.
    pub fn selected(&self) -> Option<&TranscriptSearchResult> {
        self.selected.and_then(|index| self.results.get(index))
    }

    fn advance(&mut self, direction: SearchDirection) -> Option<usize> {
        if self.results.is_empty() {
            return None;
        }
        let len = self.results.len();
        let index = match (self.selected, direction) {
            (None, SearchDirection::Next) => 0,
            (Some(index), SearchDirection::Next) => (index + 1) % len,
            (None | Some(0), SearchDirection::Previous) => len - 1,
            (Some(index), SearchDirection::Previous) => index - 1,
        };
        self.selected = Some(index);
        Some(index)
    }

    fn invalidate(&mut self) {
        self.results.clear();
        self.selected = None;
    }
}

/// Direction for moving through a [`TranscriptSearchNavigator`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Next,
    Previous,
}

impl TranscriptState {
    /// Empty transcript.
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            list: ListState::new(),
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
            pending_search_scroll: None,
            active_search_result: None,
            #[cfg(feature = "widgets")]
            tool_call_aux: Vec::new(),
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

    /// Latest plan cell in the transcript, if any.
    pub fn current_plan(&self) -> Option<&Plan> {
        self.plan_cell.and_then(|idx| {
            self.cells.get(idx).and_then(|cell| match cell {
                Cell::Plan(plan) => Some(plan),
                _ => None,
            })
        })
    }

    /// All transcript cells in order.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Mutable access to cells (for layout warm-up in the transcript widget).
    ///
    /// This advances the transcript revision before returning the slice, invalidating outstanding
    /// search results and layout measurements in case the caller changes cell content.
    pub fn cells_mut(&mut self) -> &mut [Cell] {
        self.bump();
        &mut self.cells
    }

    /// Fuzzy-search text from transcript cells, ordered by relevance.
    ///
    /// Searches user and assistant messages, thinking text, tool titles/paths/inputs/outputs and
    /// diffs, plan entries, notices, and errors. Returns no results for an empty query. The
    /// `limit` caps the result count; each match identifies one transcript cell.
    pub fn search(&self, query: &str, limit: usize) -> Vec<TranscriptSearchResult> {
        if limit == 0 {
            return Vec::new();
        }

        let mut filter = FuzzyFilter::new();
        filter.set_pattern(query);
        if filter.matches_everything() {
            return Vec::new();
        }

        let mut results = Vec::new();
        let mut indices = Vec::new();
        for (cell_index, cell) in self.cells.iter().enumerate() {
            let text = searchable_cell_text(cell);
            if let Some(score) = filter.score_with_indices(&text, &mut indices) {
                results.push(TranscriptSearchResult {
                    cell_index,
                    score,
                    preview: search_preview(&text, &indices),
                    matched_indices: indices.clone(),
                    transcript_revision: self.revision,
                });
            }
        }
        results.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.cell_index.cmp(&right.cell_index))
        });
        results.truncate(limit);
        results
    }

    /// Create a navigator over the current fuzzy search results.
    pub fn search_navigator(&self, query: &str, limit: usize) -> TranscriptSearchNavigator {
        TranscriptSearchNavigator {
            results: self.search(query, limit),
            selected: None,
            transcript_revision: self.revision,
        }
    }

    /// Move to the next or previous search result, wrapping at either end.
    ///
    /// Returns `false` for an empty or stale navigator. Stale results are discarded.
    pub fn navigate_search(
        &mut self,
        navigator: &mut TranscriptSearchNavigator,
        direction: SearchDirection,
    ) -> bool {
        if navigator.transcript_revision != self.revision {
            navigator.invalidate();
            return false;
        }
        let Some(index) = navigator.advance(direction) else {
            return false;
        };
        self.jump_to_search_result(&navigator.results[index])
    }

    /// Scroll the transcript to a current search result and highlight its cell.
    ///
    /// The scroll is applied on the next transcript render, after current row heights are
    /// measured. Returns `false` if the transcript changed since the result was produced.
    pub fn jump_to_search_result(&mut self, result: &TranscriptSearchResult) -> bool {
        if result.transcript_revision != self.revision || result.cell_index >= self.cells.len() {
            return false;
        }
        self.list.select(Some(result.cell_index));
        self.pending_search_scroll = Some(result.cell_index);
        self.active_search_result = Some(result.clone());
        true
    }

    #[cfg(feature = "widgets")]
    pub(crate) fn search_highlight(&self) -> Option<(usize, String, Vec<u32>)> {
        let result = self.active_search_result.as_ref()?;
        let cell = self.cells.get(result.cell_index)?;
        Some((
            result.cell_index,
            searchable_cell_text(cell),
            result.matched_indices.clone(),
        ))
    }

    #[cfg(feature = "widgets")]
    pub(crate) fn take_pending_search_scroll(&mut self) -> Option<usize> {
        self.pending_search_scroll.take()
    }

    /// Take the finished transcript cells, leaving session metadata intact.
    ///
    /// Returns `None` while the agent turn is still active. This is useful when a terminal
    /// renderer moves completed turns into scrollback instead of keeping them in its live view.
    pub fn take_completed_cells(&mut self) -> Option<Vec<Cell>> {
        if self.busy {
            return None;
        }

        self.open_assistant.clear();
        self.open_thinking.clear();
        self.open_tools.clear();
        self.plan_cell = None;
        self.list = ListState::new();
        #[cfg(feature = "widgets")]
        self.tool_call_aux.clear();
        self.bump();

        Some(std::mem::take(&mut self.cells))
    }

    /// Virtual list scroll state (follow-tail, offset, scrollbar layout).
    pub fn list(&self) -> &ListState {
        &self.list
    }

    /// Mutable virtual list state.
    pub fn list_mut(&mut self) -> &mut ListState {
        &mut self.list
    }

    /// Resize per-cell tool card aux state to match [`cells`](Self::cells).
    #[cfg(feature = "widgets")]
    pub fn sync_tool_call_aux(&mut self) {
        while self.tool_call_aux.len() < self.cells.len() {
            self.tool_call_aux.push(ToolCallCardAux::default());
        }
        if self.tool_call_aux.len() > self.cells.len() {
            self.tool_call_aux.truncate(self.cells.len());
        }
    }

    /// Mutable aux state for a tool-call row at `index` (after [`Self::sync_tool_call_aux`]).
    #[cfg(feature = "widgets")]
    pub fn tool_call_aux_mut(&mut self, index: usize) -> Option<&mut ToolCallCardAux> {
        self.tool_call_aux.get_mut(index)
    }

    /// Cells, list scroll, and tool aux for one transcript frame.
    #[cfg(feature = "widgets")]
    pub(crate) fn layout_borrow_mut(
        &mut self,
    ) -> (&mut [Cell], &mut ListState, &mut [ToolCallCardAux]) {
        self.sync_tool_call_aux();
        (&mut self.cells, &mut self.list, &mut self.tool_call_aux)
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
        self.pending_search_scroll = None;
        self.active_search_result = None;
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

pub(crate) fn searchable_cell_text(cell: &Cell) -> String {
    let mut text = String::new();
    match cell {
        Cell::User(user) => {
            for part in &user.content {
                match part {
                    super::types::ContentPart::Text(value) => append_search_text(&mut text, value),
                    super::types::ContentPart::Image { alt } => append_search_text(&mut text, alt),
                }
            }
        }
        Cell::Assistant(assistant) => {
            append_search_text(&mut text, assistant.stream.source());
        }
        Cell::Thinking(thinking) => {
            append_search_text(&mut text, thinking.stream.source());
        }
        Cell::ToolCall(tool) => {
            append_search_text(&mut text, tool.title());
            append_search_text(&mut text, tool.input_text());
            append_search_text(&mut text, tool.output_text());
            for location in tool.locations() {
                let path = location.to_string_lossy();
                append_search_text(&mut text, &path);
            }
            if let Some((old_text, new_text)) =
                tool.inline_diff().and_then(|diff| diff.source_texts())
            {
                append_search_text(&mut text, old_text);
                append_search_text(&mut text, new_text);
            }
        }
        Cell::Plan(plan) => {
            for entry in &plan.entries {
                append_search_text(&mut text, &entry.title);
            }
        }
        Cell::Notice(notice) => {
            append_search_text(&mut text, &notice.text);
        }
        Cell::Error(error) => {
            append_search_text(&mut text, &error.text);
        }
    }
    text
}

fn append_search_text(target: &mut String, value: &str) {
    if value.is_empty() {
        return;
    }
    if !target.is_empty() {
        target.push('\n');
    }
    target.push_str(value);
}

fn search_preview(text: &str, indices: &[u32]) -> String {
    const CONTEXT_GRAPHEMES: usize = 36;
    const MAX_PREVIEW_GRAPHEMES: usize = 120;

    let grapheme_count = text.graphemes(true).count();
    let first = indices
        .first()
        .and_then(|index| usize::try_from(*index).ok())
        .unwrap_or(0)
        .min(grapheme_count);
    let last = indices
        .last()
        .and_then(|index| usize::try_from(*index).ok())
        .and_then(|index| index.checked_add(1))
        .unwrap_or(first)
        .min(grapheme_count);
    let start = first.saturating_sub(CONTEXT_GRAPHEMES);
    let end = last
        .saturating_add(CONTEXT_GRAPHEMES)
        .min(grapheme_count)
        .min(start.saturating_add(MAX_PREVIEW_GRAPHEMES));

    let mut preview = String::with_capacity(text.len().min(256));
    if start > 0 {
        preview.push('…');
    }
    for grapheme in text.graphemes(true).skip(start).take(end - start) {
        preview.push_str(grapheme);
    }
    if end < grapheme_count {
        preview.push('…');
    }
    preview
}

impl Default for TranscriptState {
    fn default() -> Self {
        Self::new()
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
