//! Composer input state (no ratatui): mentions, slash commands, history, queue.

use std::path::PathBuf;
use std::time::Instant;

use ratakit_core::{
    Atom, AtomKind, CompletionInsert, CompletionIntent, CompletionItem, CompletionState,
    CompletionTrigger, CompletionTriggers, Component, Event, KeyCode, KeyEvent, Keymap, Outcome,
    PasteBurst, Revisioned, Scope, TextBuffer,
};

/// Minimal slash-command descriptor until A1 lands in `agent::state`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommand {
    pub name: String,
    pub description: Option<String>,
}

/// Pending attachment (images render as placeholder atoms in the buffer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub label: String,
    pub path: Option<PathBuf>,
    pub image: bool,
}

/// Application-provided file search for `@` mentions.
pub trait FileIndex: Send + Sync {
    fn query(&self, pattern: &str, limit: usize) -> Vec<PathBuf>;
}

/// High-level intents from the composer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposerIntent {
    Submit {
        text: String,
        mentions: Vec<PathBuf>,
        attachments: Vec<Attachment>,
    },
    SlashCommand {
        name: String,
        args: String,
    },
    /// Esc while the agent is busy — interrupt the in-flight turn.
    Cancel,
    OpenModelPicker,
}

/// Payload waiting while [`ComposerState::agent_busy`] is true.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueuedPayload {
    Submit {
        text: String,
        mentions: Vec<PathBuf>,
        attachments: Vec<Attachment>,
    },
    SlashCommand {
        name: String,
        args: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedMessage {
    pub payload: QueuedPayload,
}

/// Agent message composer backed by [`TextBuffer`] and [`CompletionState`].
#[derive(Debug)]
pub struct ComposerState {
    pub buffer: TextBuffer,
    pub completion: CompletionState,
    paste_burst: PasteBurst,
    slash_commands: Vec<SlashCommand>,
    history: Vec<String>,
    history_browse: Option<usize>,
    history_saved_draft: Option<String>,
    history_match_prefix: Option<String>,
    queue: Vec<QueuedMessage>,
    agent_busy: bool,
    revision: u64,
}

impl Default for ComposerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ComposerState {
    pub fn new() -> Self {
        let triggers = CompletionTriggers {
            triggers: vec![
                CompletionTrigger::new('@'),
                CompletionTrigger::first_line('/'),
            ],
        };
        Self {
            buffer: TextBuffer::new(),
            completion: CompletionState::with_triggers(triggers),
            paste_burst: PasteBurst::new(),
            slash_commands: Vec::new(),
            history: Vec::new(),
            history_browse: None,
            history_saved_draft: None,
            history_match_prefix: None,
            queue: Vec::new(),
            agent_busy: false,
            revision: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.display_len() == 0
    }

    pub fn set_agent_busy(&mut self, busy: bool) {
        if self.agent_busy != busy {
            self.agent_busy = busy;
            self.bump_revision();
        }
    }

    pub fn agent_busy(&self) -> bool {
        self.agent_busy
    }

    pub fn set_slash_commands(&mut self, commands: Vec<SlashCommand>) {
        self.slash_commands = commands;
        self.bump_revision();
    }

    pub fn slash_commands(&self) -> &[SlashCommand] {
        &self.slash_commands
    }

    pub fn queue(&self) -> &[QueuedMessage] {
        &self.queue
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn pop_queued(&mut self) -> Option<QueuedMessage> {
        if self.queue.is_empty() {
            return None;
        }
        let msg = self.queue.remove(0);
        self.bump_revision();
        Some(msg)
    }

    pub fn clear_queue(&mut self) {
        if !self.queue.is_empty() {
            self.queue.clear();
            self.bump_revision();
        }
    }

    pub fn add_attachment(&mut self, attachment: Attachment) {
        self.buffer
            .insert_atom(Atom::attachment(attachment.label.clone()));
        self.bump_revision();
    }

    pub fn block_title(&self) -> String {
        if self.queue.is_empty() {
            "composer".to_string()
        } else {
            format!("composer ({} queued)", self.queue.len())
        }
    }

    /// Refreshes the `@` / `/` completion catalog (call after buffer edits outside [`ComposerState::handle`]).
    pub fn sync_completions(&mut self, files: &dyn FileIndex) {
        self.sync_completion(files);
    }

    pub fn handle(
        &mut self,
        ev: &Event,
        keys: &Keymap,
        files: &dyn FileIndex,
    ) -> Outcome<ComposerIntent> {
        self.sync_completion(files);

        match ev {
            Event::Paste(text) => {
                self.paste_burst.on_paste(Instant::now());
                self.buffer.insert_paste(text);
                self.sync_completion(files);
                self.exit_history_browse();
                Outcome::Consumed
            }
            Event::Key(key) => self.handle_key(key, keys, files),
            Event::Tick(t) => {
                self.paste_burst.tick(*t);
                Outcome::Ignored
            }
            _ => Outcome::Ignored,
        }
    }

    fn handle_key(
        &mut self,
        key: &KeyEvent,
        keys: &Keymap,
        files: &dyn FileIndex,
    ) -> Outcome<ComposerIntent> {
        let now = Instant::now();

        if let Some(action) = keys.resolve(&[Scope::Focused, Scope::Global], key) {
            match action.0.as_str() {
                "composer.submit" | "textarea.submit" => {
                    return self.try_submit();
                }
                "composer.model_picker" => {
                    return Outcome::Intent(ComposerIntent::OpenModelPicker);
                }
                _ => {}
            }
        }

        if self.completion.is_open() {
            match self.completion.handle(&Event::Key(*key), keys) {
                Outcome::Intent(CompletionIntent::Accepted(item)) => {
                    self.completion
                        .accept_into_buffer(&mut self.buffer, Some(&item));
                    self.sync_completion(files);
                    self.exit_history_browse();
                    if matches!(item.insert, CompletionInsert::Text(ref t) if t == "/model ") {
                        return Outcome::Intent(ComposerIntent::OpenModelPicker);
                    }
                    return Outcome::Consumed;
                }
                Outcome::Intent(CompletionIntent::Dismissed) => {
                    if key.code == KeyCode::Enter {
                        return self.try_submit();
                    }
                    return Outcome::Consumed;
                }
                Outcome::Consumed => return Outcome::Consumed,
                Outcome::Ignored => {}
            }
        }

        if key.code == KeyCode::Escape && self.agent_busy {
            return Outcome::Intent(ComposerIntent::Cancel);
        }

        if !self.completion.is_open() {
            match key.code {
                KeyCode::Up => {
                    if self.history_up() {
                        return Outcome::Consumed;
                    }
                }
                KeyCode::Down => {
                    if self.history_down() {
                        return Outcome::Consumed;
                    }
                }
                _ => {}
            }
        }

        let outcome = self.handle_editing_key(key, keys, now);
        self.sync_completion(files);
        if outcome != Outcome::Ignored {
            self.exit_history_browse_on_edit(key);
        }
        outcome
    }

    fn handle_editing_key(
        &mut self,
        key: &KeyEvent,
        _keys: &Keymap,
        now: Instant,
    ) -> Outcome<ComposerIntent> {
        match key.code {
            KeyCode::Enter => {
                if self.paste_burst.enter_inserts_newline(now) {
                    self.buffer.insert_str("\n");
                    return Outcome::Consumed;
                }
                self.paste_burst.reset();
                self.try_submit()
            }
            KeyCode::Char(c) => {
                let mods = key.modifiers;
                if mods.control && c == 'z' && self.buffer.undo() {
                    return Outcome::Consumed;
                }
                if mods.control && (c == 'y' || c == 'Z') && self.buffer.redo() {
                    return Outcome::Consumed;
                }
                self.paste_burst.on_char(now);
                self.buffer.insert_str(&c.to_string());
                Outcome::Consumed
            }
            KeyCode::Backspace => {
                self.paste_burst.on_other_key(now);
                self.buffer.delete_backward();
                Outcome::Consumed
            }
            KeyCode::Delete => {
                self.paste_burst.on_other_key(now);
                self.buffer.delete_forward();
                Outcome::Consumed
            }
            KeyCode::Left => {
                self.paste_burst.on_other_key(now);
                self.buffer.move_left();
                Outcome::Consumed
            }
            KeyCode::Right => {
                self.paste_burst.on_other_key(now);
                self.buffer.move_right();
                Outcome::Consumed
            }
            KeyCode::Up => {
                self.paste_burst.on_other_key(now);
                self.buffer.move_up();
                Outcome::Consumed
            }
            KeyCode::Down => {
                self.paste_burst.on_other_key(now);
                self.buffer.move_down();
                Outcome::Consumed
            }
            _ => {
                self.paste_burst.on_other_key(now);
                Outcome::Ignored
            }
        }
    }

    fn try_submit(&mut self) -> Outcome<ComposerIntent> {
        let text = self.buffer.text();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Outcome::Consumed;
        }

        if let Some((name, args)) = parse_slash_command(trimmed) {
            if name == "model" && args.is_empty() {
                self.clear_input();
                return Outcome::Intent(ComposerIntent::OpenModelPicker);
            }
            if self.agent_busy {
                self.queue.push(QueuedMessage {
                    payload: QueuedPayload::SlashCommand {
                        name: name.to_string(),
                        args: args.to_string(),
                    },
                });
                self.clear_input();
                return Outcome::Consumed;
            }
            self.clear_input();
            return Outcome::Intent(ComposerIntent::SlashCommand {
                name: name.to_string(),
                args: args.to_string(),
            });
        }

        let mentions = collect_mentions(&self.buffer);
        let attachments = collect_attachments(&self.buffer);

        if self.agent_busy {
            self.queue.push(QueuedMessage {
                payload: QueuedPayload::Submit {
                    text: text.clone(),
                    mentions: mentions.clone(),
                    attachments: attachments.clone(),
                },
            });
            self.clear_input();
            return Outcome::Consumed;
        }

        self.push_history(text.clone());
        self.clear_input();
        Outcome::Intent(ComposerIntent::Submit {
            text,
            mentions,
            attachments,
        })
    }

    fn clear_input(&mut self) {
        self.buffer.set_text("");
        self.completion.dismiss();
        self.exit_history_browse();
        self.bump_revision();
    }

    fn push_history(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        if self.history.last() == Some(&text) {
            return;
        }
        self.history.push(text);
    }

    fn matching_history_indices(&self) -> Vec<usize> {
        let prefix = self
            .history_match_prefix
            .as_deref()
            .map_or_else(|| self.buffer.text(), String::from);
        self.history
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.starts_with(&prefix))
            .map(|(idx, _)| idx)
            .collect()
    }

    fn history_up(&mut self) -> bool {
        let matches = self.matching_history_indices();
        if matches.is_empty() {
            return false;
        }
        if self.history_browse.is_none() {
            self.history_saved_draft = Some(self.buffer.text());
            self.history_match_prefix = Some(self.buffer.text());
        }
        let current = self.history_browse.unwrap_or(matches.len());
        let pos = matches
            .iter()
            .position(|&idx| idx == current)
            .map(|p| p.saturating_sub(1))
            .unwrap_or(matches.len() - 1);
        let next_idx = matches[pos];
        self.history_browse = Some(next_idx);
        self.buffer.set_text(&self.history[next_idx]);
        self.bump_revision();
        true
    }

    fn history_down(&mut self) -> bool {
        if self.history_browse.is_none() {
            return false;
        }
        let matches = self.matching_history_indices();
        let current = self.history_browse.expect("browse");
        if let Some(p) = matches.iter().position(|&idx| idx == current) {
            if p + 1 < matches.len() {
                let next_idx = matches[p + 1];
                self.history_browse = Some(next_idx);
                self.buffer.set_text(&self.history[next_idx]);
                self.bump_revision();
                return true;
            }
        }
        self.exit_history_browse();
        true
    }

    fn exit_history_browse(&mut self) {
        if self.history_browse.is_none() {
            return;
        }
        if let Some(draft) = self.history_saved_draft.take() {
            self.buffer.set_text(&draft);
        }
        self.history_browse = None;
        self.history_match_prefix = None;
        self.bump_revision();
    }

    fn exit_history_browse_on_edit(&mut self, key: &KeyEvent) {
        if self.history_browse.is_none() {
            return;
        }
        match key.code {
            KeyCode::Up | KeyCode::Down => {}
            _ => {
                self.history_browse = None;
                self.history_saved_draft = None;
                self.history_match_prefix = None;
            }
        }
    }

    fn sync_completion(&mut self, files: &dyn FileIndex) {
        self.completion.sync_from_buffer(&self.buffer);
        self.refresh_completion_catalog(files);
    }

    fn refresh_completion_catalog(&mut self, files: &dyn FileIndex) {
        let items = match self.completion.session() {
            Some(session) if session.trigger == '@' => file_completion_items(files, &session.query),
            Some(session) if session.trigger == '/' => self.slash_completion_items(),
            _ => Vec::new(),
        };
        if catalog_changed(self.completion.catalog(), &items) {
            self.completion.set_catalog(items);
        }
    }

    fn slash_completion_items(&self) -> Vec<CompletionItem> {
        self.slash_commands
            .iter()
            .map(|cmd| {
                let label = format!("/{}", cmd.name);
                let insert = CompletionInsert::Text(format!("/{} ", cmd.name));
                let item = CompletionItem::new(label, insert);
                if let Some(desc) = &cmd.description {
                    item.with_detail(desc.clone())
                } else {
                    item
                }
            })
            .collect()
    }

    fn bump_revision(&mut self) {
        self.revision += 1;
    }
}

impl Revisioned for ComposerState {
    fn revision(&self) -> u64 {
        self.revision + self.buffer.revision() + Component::revision(&self.completion)
    }
}

fn parse_slash_command(text: &str) -> Option<(&str, &str)> {
    if !text.starts_with('/') {
        return None;
    }
    let rest = text.get(1..)?.trim_start();
    if rest.is_empty() {
        return None;
    }
    match rest.split_once(char::is_whitespace) {
        Some((name, args)) => Some((name, args.trim())),
        None => Some((rest, "")),
    }
}

fn collect_mentions(buffer: &TextBuffer) -> Vec<PathBuf> {
    buffer
        .atoms()
        .iter()
        .filter_map(|atom| match &atom.kind {
            AtomKind::Mention { label } => {
                let path = label.trim_start_matches('@');
                Some(PathBuf::from(path))
            }
            _ => None,
        })
        .collect()
}

fn collect_attachments(buffer: &TextBuffer) -> Vec<Attachment> {
    buffer
        .atoms()
        .iter()
        .filter_map(|atom| match &atom.kind {
            AtomKind::Attachment { label } => {
                let lower = label.to_ascii_lowercase();
                Some(Attachment {
                    label: label.clone(),
                    path: None,
                    image: lower.ends_with(".png")
                        || lower.ends_with(".jpg")
                        || lower.ends_with(".jpeg")
                        || lower.ends_with(".gif")
                        || lower.ends_with(".webp"),
                })
            }
            _ => None,
        })
        .collect()
}

fn file_completion_items(files: &dyn FileIndex, query: &str) -> Vec<CompletionItem> {
    files
        .query(query, 64)
        .into_iter()
        .map(|path| {
            let label = format!("@{}", path.display());
            CompletionItem::new(
                label.clone(),
                CompletionInsert::Mention {
                    label: label.clone(),
                },
            )
        })
        .collect()
}

fn catalog_changed(current: &[CompletionItem], next: &[CompletionItem]) -> bool {
    current.len() != next.len()
        || current
            .iter()
            .zip(next.iter())
            .any(|(a, b)| a.label != b.label || a.insert != b.insert)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratakit_core::{Keymap, Revisioned};

    struct StaticFiles(Vec<PathBuf>);

    impl FileIndex for StaticFiles {
        fn query(&self, pattern: &str, limit: usize) -> Vec<PathBuf> {
            self.0
                .iter()
                .filter(|p| p.to_string_lossy().contains(pattern))
                .take(limit)
                .cloned()
                .collect()
        }
    }

    fn enter_key() -> Event {
        Event::Key(KeyEvent::new(KeyCode::Enter))
    }

    #[test]
    fn submit_emits_text_and_mentions() {
        let mut composer = ComposerState::new();
        composer.buffer.insert_str("see ");
        composer.buffer.insert_atom(Atom::mention("@src/main.rs"));
        let outcome = composer.handle(&enter_key(), &Keymap::empty(), &StaticFiles(vec![]));
        assert!(matches!(
            outcome,
            Outcome::Intent(ComposerIntent::Submit {
                mentions,
                ..
            }) if mentions == vec![PathBuf::from("src/main.rs")]
        ));
        assert!(composer.is_empty());
    }

    #[test]
    fn slash_command_intent() {
        let mut composer = ComposerState::new();
        composer.buffer.insert_str("/clear now");
        let outcome = composer.handle(&enter_key(), &Keymap::empty(), &StaticFiles(vec![]));
        assert_eq!(
            outcome,
            Outcome::Intent(ComposerIntent::SlashCommand {
                name: "clear".into(),
                args: "now".into(),
            })
        );
    }

    #[test]
    fn busy_escape_cancels() {
        let mut composer = ComposerState::new();
        composer.set_agent_busy(true);
        let esc = Event::Key(KeyEvent::new(KeyCode::Escape));
        let outcome = composer.handle(&esc, &Keymap::empty(), &StaticFiles(vec![]));
        assert_eq!(outcome, Outcome::Intent(ComposerIntent::Cancel));
    }

    #[test]
    fn queue_while_busy() {
        let mut composer = ComposerState::new();
        composer.set_agent_busy(true);
        composer.buffer.insert_str("hello");
        let _ = composer.handle(&enter_key(), &Keymap::empty(), &StaticFiles(vec![]));
        assert_eq!(composer.queue_len(), 1);
        assert!(composer.is_empty());
        let msg = composer.pop_queued().expect("queued");
        assert_eq!(
            msg.payload,
            QueuedPayload::Submit {
                text: "hello".into(),
                mentions: vec![],
                attachments: vec![],
            }
        );
    }

    #[test]
    fn history_prefix_navigation() {
        let mut composer = ComposerState::new();
        composer.buffer.insert_str("cargo test");
        let _ = composer.handle(&enter_key(), &Keymap::empty(), &StaticFiles(vec![]));
        composer.buffer.insert_str("cargo fmt");
        let _ = composer.handle(&enter_key(), &Keymap::empty(), &StaticFiles(vec![]));
        composer.buffer.insert_str("cargo ");
        let up = Event::Key(KeyEvent::new(KeyCode::Up));
        let keys = Keymap::empty();
        let files = StaticFiles(vec![]);
        assert_eq!(composer.handle(&up, &keys, &files), Outcome::Consumed);
        assert_eq!(composer.buffer.text(), "cargo fmt");
        assert_eq!(composer.handle(&up, &keys, &files), Outcome::Consumed);
        assert_eq!(composer.buffer.text(), "cargo test");
    }

    #[test]
    fn large_paste_becomes_atom() {
        let mut composer = ComposerState::new();
        let paste = "line\n".repeat(12);
        composer.handle(&Event::Paste(paste), &Keymap::empty(), &StaticFiles(vec![]));
        assert_eq!(composer.buffer.text(), "[Pasted 12 lines]");
    }

    #[test]
    fn revision_increases_on_edit() {
        let mut composer = ComposerState::new();
        let before = composer.revision();
        composer.buffer.insert_str("x");
        assert!(composer.revision() > before);
    }
}
