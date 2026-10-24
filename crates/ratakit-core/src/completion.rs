// Derived from openai/codex codex-rs/tui/src/bottom_pane/command_popup.rs (Apache-2.0).
// Copyright 2025 OpenAI. Licensed under the Apache License, Version 2.0.
// Modifications Copyright 2026 The ratakit Authors.

use crate::component::Component;
use crate::event::{Event, KeyCode, KeyEvent};
use crate::fuzzy::{FuzzyFilter, FuzzyRankedMatch};
use crate::keymap::Keymap;
use crate::outcome::Outcome;
use crate::text::{Atom, Position, TextBuffer};

/// How accepted values are written into a [`TextBuffer`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionInsert {
    /// Plain UTF-8 text (slash commands, snippets).
    Text(String),
    /// `@`-style mention rendered as an [`Atom`].
    Mention { label: String },
    /// Arbitrary atom (attachments, collapsed paste markers, …).
    Atom(Atom),
}

/// One selectable completion row supplied by the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub insert: CompletionInsert,
}

impl CompletionItem {
    pub fn new(label: impl Into<String>, insert: CompletionInsert) -> Self {
        Self {
            label: label.into(),
            detail: None,
            insert,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Trigger character and matching rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionTrigger {
    pub ch: char,
    /// When `true`, the trigger is only considered on the first line (`/` commands).
    pub first_line_only: bool,
}

impl CompletionTrigger {
    pub const fn new(ch: char) -> Self {
        Self {
            ch,
            first_line_only: false,
        }
    }

    pub const fn first_line(ch: char) -> Self {
        Self {
            ch,
            first_line_only: true,
        }
    }
}

/// Default `@`, `/`, and `#` triggers from the design doc.
#[derive(Debug, Clone)]
pub struct CompletionTriggers {
    pub triggers: Vec<CompletionTrigger>,
}

impl Default for CompletionTriggers {
    fn default() -> Self {
        Self {
            triggers: vec![
                CompletionTrigger::new('@'),
                CompletionTrigger::first_line('/'),
                CompletionTrigger::new('#'),
            ],
        }
    }
}

impl CompletionTriggers {
    pub fn contains(&self, ch: char) -> bool {
        self.triggers.iter().any(|t| t.ch == ch)
    }

    fn trigger_for(&self, ch: char) -> Option<&CompletionTrigger> {
        self.triggers.iter().find(|t| t.ch == ch)
    }
}

/// Active completion span inside the buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionSession {
    pub trigger: char,
    /// Byte index of the trigger character in [`TextBuffer::display`].
    pub start: Position,
    /// Filter query (text after the trigger up to the cursor).
    pub query: String,
}

/// Popup width relative to the anchor [`TextArea`](../../ratakit-widgets) inner area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompletionMenuWidth {
    /// Span the full inner width of the text area (default per product decision).
    #[default]
    Full,
}

/// Vertical placement of the menu relative to the text cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompletionPlacement {
    Above,
    #[default]
    Below,
}

/// Intents surfaced when the menu consumes navigation keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionIntent {
    Accepted(CompletionItem),
    Dismissed,
}

/// Core state for an anchored completion menu (no ratatui).
#[derive(Debug)]
pub struct CompletionState {
    triggers: CompletionTriggers,
    catalog: Vec<CompletionItem>,
    filter: FuzzyFilter,
    session: Option<CompletionSession>,
    ranked: Vec<FuzzyRankedMatch>,
    selected: usize,
    list_scroll: usize,
    max_visible: usize,
    revision: u64,
}

impl Default for CompletionState {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionState {
    pub fn new() -> Self {
        Self::with_triggers(CompletionTriggers::default())
    }

    pub fn with_triggers(triggers: CompletionTriggers) -> Self {
        Self {
            triggers,
            catalog: Vec::new(),
            filter: FuzzyFilter::new(),
            session: None,
            ranked: Vec::new(),
            selected: 0,
            list_scroll: 0,
            max_visible: 8,
            revision: 0,
        }
    }

    pub fn set_catalog(&mut self, items: Vec<CompletionItem>) {
        self.catalog = items;
        self.refresh_ranked();
        self.bump_revision();
    }

    pub fn catalog(&self) -> &[CompletionItem] {
        &self.catalog
    }

    pub fn triggers(&self) -> &CompletionTriggers {
        &self.triggers
    }

    pub fn set_max_visible_rows(&mut self, rows: usize) {
        self.max_visible = rows.max(1);
        self.ensure_scroll_visible();
        self.bump_revision();
    }

    pub fn max_visible_rows(&self) -> usize {
        self.max_visible
    }

    pub fn is_open(&self) -> bool {
        self.session.is_some()
    }

    pub fn session(&self) -> Option<&CompletionSession> {
        self.session.as_ref()
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn list_scroll_offset(&self) -> usize {
        self.list_scroll
    }

    pub fn ranked_matches(&self) -> &[FuzzyRankedMatch] {
        &self.ranked
    }

    pub fn selected_item(&self) -> Option<&CompletionItem> {
        let idx = self.ranked.get(self.selected)?.item_index;
        self.catalog.get(idx)
    }

    /// Recompute the active session from the buffer cursor and refresh matches.
    pub fn sync_from_buffer(&mut self, buffer: &TextBuffer) {
        let previous = self.session.clone();
        self.session = detect_session(buffer, &self.triggers);
        if self.session != previous {
            self.selected = 0;
            self.list_scroll = 0;
        }
        let query = self
            .session
            .as_ref()
            .map(|s| s.query.as_str())
            .unwrap_or("");
        if self.filter.pattern() != query {
            self.filter.set_pattern(query);
        }
        self.refresh_ranked();
        self.clamp_selection();
        if previous != self.session {
            self.bump_revision();
        }
    }

    pub fn dismiss(&mut self) {
        if self.session.is_none() {
            return;
        }
        self.session = None;
        self.ranked.clear();
        self.selected = 0;
        self.list_scroll = 0;
        self.filter.set_pattern("");
        self.bump_revision();
    }

    pub fn select_next(&mut self) {
        if self.ranked.is_empty() {
            return;
        }
        let len = self.ranked.len();
        self.selected = (self.selected + 1) % len;
        self.ensure_scroll_visible();
        self.bump_revision();
    }

    pub fn select_prev(&mut self) {
        if self.ranked.is_empty() {
            return;
        }
        let len = self.ranked.len();
        self.selected = (self.selected + len - 1) % len;
        self.ensure_scroll_visible();
        self.bump_revision();
    }

    /// Replace the active trigger span with the selected item, or `item` when provided.
    pub fn accept_into_buffer(
        &mut self,
        buffer: &mut TextBuffer,
        item: Option<&CompletionItem>,
    ) -> Option<CompletionItem> {
        let session = self.session.clone()?;
        let chosen = item.cloned().or_else(|| self.selected_item().cloned())?;
        let end = buffer.cursor();
        let range = session.start..end;
        match &chosen.insert {
            CompletionInsert::Text(text) => buffer.replace_range(range, text.as_str(), None),
            CompletionInsert::Mention { label } => {
                buffer.replace_range(range, "", Some(Atom::mention(label.clone())));
            }
            CompletionInsert::Atom(atom) => {
                buffer.replace_range(range, "", Some(atom.clone()));
            }
        }
        self.dismiss();
        Some(chosen)
    }
}

impl Component for CompletionState {
    type Intent = CompletionIntent;

    fn handle(&mut self, ev: &Event, _keys: &Keymap) -> Outcome<Self::Intent> {
        if !self.is_open() {
            return Outcome::Ignored;
        }
        if let Event::Key(key) = ev {
            return self.handle_key(key);
        }
        Outcome::Ignored
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}

impl CompletionState {
    fn handle_key(&mut self, key: &KeyEvent) -> Outcome<CompletionIntent> {
        match key.code {
            KeyCode::Down => {
                self.select_next();
                Outcome::Consumed
            }
            KeyCode::Up => {
                self.select_prev();
                Outcome::Consumed
            }
            KeyCode::Escape => {
                self.dismiss();
                Outcome::Intent(CompletionIntent::Dismissed)
            }
            KeyCode::Enter | KeyCode::Tab => {
                if let Some(item) = self.selected_item().cloned() {
                    Outcome::Intent(CompletionIntent::Accepted(item))
                } else {
                    self.dismiss();
                    Outcome::Intent(CompletionIntent::Dismissed)
                }
            }
            _ => Outcome::Ignored,
        }
    }

    fn refresh_ranked(&mut self) {
        if self.session.is_none() {
            self.ranked.clear();
            return;
        }
        let labels: Vec<String> = self.catalog.iter().map(|i| i.label.clone()).collect();
        let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        self.ranked = self.filter.rank(&label_refs, self.catalog.len());
    }

    fn clamp_selection(&mut self) {
        if self.ranked.is_empty() {
            self.selected = 0;
            self.list_scroll = 0;
            return;
        }
        self.selected = self.selected.min(self.ranked.len() - 1);
        self.ensure_scroll_visible();
    }

    fn ensure_scroll_visible(&mut self) {
        if self.ranked.is_empty() {
            self.list_scroll = 0;
            return;
        }
        if self.selected < self.list_scroll {
            self.list_scroll = self.selected;
        }
        if self.selected >= self.list_scroll + self.max_visible {
            self.list_scroll = self.selected + 1 - self.max_visible;
        }
    }

    fn bump_revision(&mut self) {
        self.revision += 1;
    }
}

fn detect_session(buffer: &TextBuffer, triggers: &CompletionTriggers) -> Option<CompletionSession> {
    let cursor = buffer.cursor();
    let display = buffer.display();
    let (line, _) = line_col_at(display.as_str(), cursor);
    let line_start = line_start_at(display.as_str(), line);
    let line_end = line_end_at(display.as_str(), line_start);
    let line_text = display.get(line_start..line_end).unwrap_or("");
    let rel_cursor = cursor - line_start;

    let before = line_text.get(..rel_cursor).unwrap_or(line_text);
    let token_start = before
        .char_indices()
        .rev()
        .find_map(|(idx, ch)| {
            if ch.is_whitespace() {
                Some(idx + ch.len_utf8())
            } else {
                None
            }
        })
        .unwrap_or(0);

    let token = before.get(token_start..).unwrap_or("");
    let first = token.chars().next()?;
    if !triggers.contains(first) {
        return None;
    }
    let rule = triggers.trigger_for(first)?;
    if rule.first_line_only && line != 0 {
        return None;
    }
    let query = token.get(first.len_utf8()..).unwrap_or("").to_string();
    let start = line_start + token_start;
    Some(CompletionSession {
        trigger: first,
        start,
        query,
    })
}

fn line_col_at(text: &str, pos: usize) -> (usize, usize) {
    let before = text.get(..pos).unwrap_or(text);
    let line = before.matches('\n').count();
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = pos - line_start;
    (line, col)
}

fn line_start_at(text: &str, line: usize) -> usize {
    if line == 0 {
        return 0;
    }
    let mut count = 0;
    for (idx, _) in text.match_indices('\n') {
        count += 1;
        if count == line {
            return idx + 1;
        }
    }
    text.len()
}

fn line_end_at(text: &str, line_start: usize) -> usize {
    let rest = text.get(line_start..).unwrap_or("");
    line_start + rest.find('\n').unwrap_or(rest.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_at_mention_token() {
        let mut buffer = TextBuffer::new();
        buffer.insert_str("hello @src/f");
        let mut state = CompletionState::new();
        state.sync_from_buffer(&buffer);
        let session = state.session().expect("session");
        assert_eq!(session.trigger, '@');
        assert_eq!(session.query, "src/f");
    }

    #[test]
    fn slash_only_on_first_line() {
        let mut buffer = TextBuffer::new();
        buffer.insert_str("/cle");
        let mut state = CompletionState::new();
        state.sync_from_buffer(&buffer);
        assert!(state.is_open());
        buffer.insert_str("\n/");
        state.sync_from_buffer(&buffer);
        assert!(!state.is_open());
    }

    #[test]
    fn accept_inserts_mention_atom() {
        let mut buffer = TextBuffer::new();
        buffer.insert_str("@fi");
        let item = CompletionItem::new(
            "@file.rs",
            CompletionInsert::Mention {
                label: "@file.rs".to_string(),
            },
        );
        let mut state = CompletionState::new();
        state.set_catalog(vec![item.clone()]);
        state.sync_from_buffer(&buffer);
        state.accept_into_buffer(&mut buffer, Some(&item));
        assert_eq!(buffer.text(), "@file.rs");
        assert_eq!(buffer.atoms().len(), 1);
        assert!(!state.is_open());
    }
}
