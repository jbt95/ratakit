//! Reference agent shell: transcript, composer, usage bar, plan side panel, and overlays.

use std::path::PathBuf;

use ratakit_core::{
    Component, Event, FocusRing, Keymap, ListState, Outcome, Revisioned, Scope, Theme,
};
use ratakit_text::{Highlighter, wrap_lines};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Layout, Rect};
use ratatui_core::widgets::{StatefulWidget, Widget};

use crate::state::composer::FileIndex;
use crate::state::{
    AgentEvent, Attachment, ComposerIntent, ComposerState, ModelCatalogEntry, ModelChoice,
    ModeId, PermissionIntent, PermissionRequest, Plan, Usage,
};
use crate::state::TranscriptState;
use crate::widgets::composer::ComposerWidgetState;
use crate::widgets::{
    Composer, ModelPicker, ModelPickerIntent, ModelPickerState, PermissionPrompt,
    PermissionPromptModal, PermissionPromptState, PlanView, Transcript, UsageBar,
};

/// Focus targets for [`AgentLayoutState`].
pub mod focus {
    use ratakit_core::FocusId;

    pub const TRANSCRIPT: FocusId = FocusId(1);
    pub const COMPOSER: FocusId = FocusId(2);
    pub const PERMISSION_MODAL: FocusId = FocusId(10);
    pub const MODEL_PICKER: FocusId = FocusId(11);
}

/// High-level intents produced by the reference agent layout.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentLayoutIntent {
    Submit {
        text: String,
        mentions: Vec<PathBuf>,
        attachments: Vec<Attachment>,
    },
    SlashCommand {
        name: String,
        args: String,
    },
    Cancel,
    Permission(PermissionIntent),
    ModelChange(ModelChoice),
}

/// Application state for [`AgentLayout`]: transcript, composer, overlays, and focus.
#[derive(Debug)]
pub struct AgentLayoutState {
    pub transcript: TranscriptState,
    pub composer: (ComposerState, ComposerWidgetState),
    pub model_picker: ModelPickerState,
    pub permission: PermissionPromptState,
    permission_request: Option<PermissionRequest>,
    pub plan_list: ListState,
    pub focus: FocusRing,
    pub model_choice: Option<ModelChoice>,
    model_catalog: Vec<ModelCatalogEntry>,
    usage: Usage,
    mode: ModeId,
    git_branch: Option<String>,
    last_width: u16,
}

impl Default for AgentLayoutState {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentLayoutState {
    pub fn new() -> Self {
        Self {
            transcript: TranscriptState::new(),
            composer: (ComposerState::new(), ComposerWidgetState::default()),
            model_picker: ModelPickerState::new(),
            permission: PermissionPromptState::new(),
            permission_request: None,
            plan_list: ListState::new(),
            focus: FocusRing::new([focus::TRANSCRIPT, focus::COMPOSER]),
            model_choice: None,
            model_catalog: Vec::new(),
            usage: Usage::default(),
            mode: ModeId::new("auto"),
            git_branch: None,
            last_width: 80,
        }
    }

    pub fn set_git_branch(&mut self, branch: Option<String>) {
        self.git_branch = branch;
    }

    pub fn set_model_catalog(&mut self, catalog: Vec<ModelCatalogEntry>) {
        self.model_catalog = catalog;
        self.model_picker.set_catalog(self.model_catalog.clone());
    }

    pub fn permission_request(&self) -> Option<&PermissionRequest> {
        self.permission_request.as_ref()
    }

    pub fn is_narrow(&self) -> bool {
        self.last_width < 56
    }

    pub fn apply_agent_event(&mut self, ev: AgentEvent) {
        self.transcript.apply(ev);
        self.sync_from_transcript();
    }

    pub fn present_permission(&mut self, request: PermissionRequest) {
        self.permission_request = Some(request);
        if let Some(req) = self.permission_request.as_ref() {
            self.permission
                .open_with_trap(req, &mut self.focus, focus::PERMISSION_MODAL);
        }
    }

    pub fn handle(
        &mut self,
        ev: &Event,
        keys: &Keymap,
        files: &dyn FileIndex,
    ) -> Outcome<AgentLayoutIntent> {
        if let Event::Resize { width, .. } = ev {
            self.last_width = *width;
        }

        if self.permission.is_open() {
            let outcome = self.permission.handle(ev, keys);
            return self.map_permission(outcome);
        }

        if self.model_picker.is_open() {
            let outcome = self.model_picker.handle(ev, keys);
            return self.map_model_picker(outcome);
        }

        if let Event::Key(key) = ev {
            if let Some(action) = keys.resolve(&[Scope::Focused, Scope::Global], key) {
                match action.0.as_str() {
                    "focus.next" => {
                        self.focus.next();
                        return Outcome::Consumed;
                    }
                    "focus.prev" => {
                        self.focus.prev();
                        return Outcome::Consumed;
                    }
                    _ => {}
                }
            }
        }

        if self.focus.focused() == Some(focus::TRANSCRIPT) {
            match self.transcript.handle(ev, keys) {
                Outcome::Intent(()) | Outcome::Consumed => return Outcome::Consumed,
                Outcome::Ignored => {}
            }
        }

        match self.composer.0.handle(ev, keys, files) {
            Outcome::Intent(intent) => self.map_composer_intent(intent),
            Outcome::Consumed => Outcome::Consumed,
            Outcome::Ignored => Outcome::Ignored,
        }
    }

    fn map_permission(&mut self, outcome: Outcome<PermissionIntent>) -> Outcome<AgentLayoutIntent> {
        match outcome {
            Outcome::Intent(intent) => {
                self.permission.close_with_trap(&mut self.focus);
                self.permission_request = None;
                Outcome::Intent(AgentLayoutIntent::Permission(intent))
            }
            Outcome::Consumed => Outcome::Consumed,
            Outcome::Ignored => Outcome::Ignored,
        }
    }

    fn map_model_picker(
        &mut self,
        outcome: Outcome<ModelPickerIntent>,
    ) -> Outcome<AgentLayoutIntent> {
        match outcome {
            Outcome::Intent(ModelPickerIntent::Selected(choice)) => {
                self.model_picker.close_with_trap(&mut self.focus);
                self.model_choice = Some(choice.clone());
                Outcome::Intent(AgentLayoutIntent::ModelChange(choice))
            }
            Outcome::Intent(ModelPickerIntent::Cancelled) => {
                self.model_picker.close_with_trap(&mut self.focus);
                Outcome::Consumed
            }
            Outcome::Consumed => Outcome::Consumed,
            Outcome::Ignored => Outcome::Ignored,
        }
    }

    fn map_composer_intent(
        &mut self,
        intent: ComposerIntent,
    ) -> Outcome<AgentLayoutIntent> {
        match intent {
            ComposerIntent::Submit {
                text,
                mentions,
                attachments,
            } => Outcome::Intent(AgentLayoutIntent::Submit {
                text,
                mentions,
                attachments,
            }),
            ComposerIntent::SlashCommand { name, args } => {
                Outcome::Intent(AgentLayoutIntent::SlashCommand { name, args })
            }
            ComposerIntent::Cancel => Outcome::Intent(AgentLayoutIntent::Cancel),
            ComposerIntent::OpenModelPicker => {
                self.refresh_model_picker_catalog();
                self.model_picker
                    .open_with_trap(&mut self.focus, focus::MODEL_PICKER);
                Outcome::Consumed
            }
        }
    }

    fn sync_from_transcript(&mut self) {
        self.composer
            .0
            .set_slash_commands(self.transcript.slash_commands().to_vec());
        self.composer.0.set_agent_busy(self.transcript.is_busy());
        if let Some(mode) = self.transcript.mode() {
            self.mode = mode.clone();
        }
        if let Some(usage) = self.transcript.usage() {
            self.usage = usage.clone();
        }
        if !self.transcript.model_choices().is_empty() {
            self.model_catalog = catalog_from_choices(self.transcript.model_choices());
            self.model_picker.set_catalog(self.model_catalog.clone());
        }
    }

    fn refresh_model_picker_catalog(&mut self) {
        if self.model_catalog.is_empty() {
            self.model_catalog = catalog_from_choices(self.transcript.model_choices());
        }
        self.model_picker.set_catalog(self.model_catalog.clone());
        self.model_picker.set_current(self.model_choice.clone());
    }

    pub(crate) fn usage(&self) -> &Usage {
        &self.usage
    }

    pub(crate) fn mode(&self) -> &ModeId {
        &self.mode
    }

    pub(crate) fn git_branch(&self) -> Option<&str> {
        self.git_branch.as_deref()
    }

    pub(crate) fn current_plan(&self) -> Option<&Plan> {
        self.transcript.current_plan()
    }

    pub(crate) fn show_plan_side_panel(&self) -> bool {
        self.last_width >= 72 && self.current_plan().is_some()
    }
}

impl Revisioned for AgentLayoutState {
    fn revision(&self) -> u64 {
        Revisioned::revision(&self.transcript)
            + Revisioned::revision(&self.composer.0)
            + Revisioned::revision(&self.model_picker)
            + Revisioned::revision(&self.permission)
            + Revisioned::revision(&self.focus)
    }
}

fn catalog_from_choices(choices: &[ModelChoice]) -> Vec<ModelCatalogEntry> {
    choices
        .iter()
        .map(|choice| {
            let mut entry =
                ModelCatalogEntry::new(choice.provider.clone(), choice.model.clone());
            if let Some(effort) = choice.effort {
                entry = entry.with_efforts(vec![effort]);
            }
            entry
        })
        .collect()
}

/// Reference layout composing transcript, composer, usage bar, plan panel, and overlays.
pub struct AgentLayout<'a> {
    theme: &'a Theme,
    highlighter: &'a dyn Highlighter,
    files: &'a dyn FileIndex,
    composer_placeholder: Option<&'a str>,
    composer_max_lines: u16,
}

impl<'a> AgentLayout<'a> {
    pub fn new(theme: &'a Theme, highlighter: &'a dyn Highlighter, files: &'a dyn FileIndex) -> Self {
        Self {
            theme,
            highlighter,
            files,
            composer_placeholder: None,
            composer_max_lines: 8,
        }
    }

    #[must_use]
    pub fn composer_placeholder(mut self, text: &'a str) -> Self {
        self.composer_placeholder = Some(text);
        self
    }

    #[must_use]
    pub fn composer_max_lines(mut self, lines: u16) -> Self {
        self.composer_max_lines = lines;
        self
    }
}

impl StatefulWidget for AgentLayout<'_> {
    type State = AgentLayoutState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut AgentLayoutState) {
        if area.is_empty() {
            return;
        }
        state.last_width = area.width;

        let usage_h = 1u16;
        let composer_h =
            measure_composer_height(&state.composer.0, area.width, self.composer_max_lines);
        let main_h = area.height.saturating_sub(usage_h + composer_h);

        let main_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: main_h,
        };

        let plan_snapshot = state.current_plan().cloned();
        let transcript_area = if state.show_plan_side_panel() {
            let chunks = Layout::horizontal([
                Constraint::Min(24),
                Constraint::Length(28),
            ])
                .split(main_area);
            if let Some(plan) = plan_snapshot.as_ref() {
                let plan_widget = PlanView::new(plan, self.theme).scrollbar(true);
                plan_widget.render(chunks[1], buf, &mut state.plan_list);
            }
            chunks[0]
        } else {
            main_area
        };

        Transcript::new(self.theme, self.highlighter)
            .render(transcript_area, buf, &mut state.transcript);

        let composer_area = Rect {
            x: area.x,
            y: area.y + main_h,
            width: area.width,
            height: composer_h,
        };
        let composer = Composer::new(self.theme, self.files)
            .placeholder(
                self.composer_placeholder
                    .unwrap_or("Type a message… (Tab: focus, Ctrl+M: model)"),
            )
            .max_height(composer_h.saturating_sub(2).max(1));
        composer.render(composer_area, buf, &mut state.composer);

        let usage_area = Rect {
            x: area.x,
            y: area.y + main_h + composer_h,
            width: area.width,
            height: usage_h,
        };
        UsageBar::new(state.usage(), state.mode(), self.theme)
            .git_branch(state.git_branch())
            .compact(state.is_narrow())
            .render(usage_area, buf);

        if let Some(request) = state.permission_request.as_ref() {
            let prompt = PermissionPrompt::new(request, self.theme, self.highlighter);
            PermissionPromptModal::new(prompt).render(area, buf, &mut state.permission);
        }

        if state.model_picker.is_open() {
            ModelPicker::new(self.theme).render(area, buf, &mut state.model_picker);
        }
    }
}

fn measure_composer_height(composer: &ComposerState, width: u16, max_lines: u16) -> u16 {
    let inner_w = width.saturating_sub(2).max(1);
    let wrapped = wrap_lines(&composer.buffer.display(), inner_w);
    let line_count = u16::try_from(wrapped.len().max(1)).unwrap_or(u16::MAX);
    let inner_h = line_count.min(max_lines);
    inner_h + 2
}
