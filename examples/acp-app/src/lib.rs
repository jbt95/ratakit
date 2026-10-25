//! Shared terminal app for the Claude Code and OpenCode ACP examples.

use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Stdout, stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use ratakit::acp::{AcpBridge, AcpBridgeHandle, AgentCommand, BridgeEvent, PermissionRequestId};
use ratakit::agent::recording::RecordingWriter;
use ratakit::agent::state::{AgentEvent, Cell, FileIndex, Level, MessageId};
use ratakit::agent::widgets::{
    AgentLayout, AgentLayoutIntent, AgentLayoutState, TranscriptHistory,
};
use ratakit::core::{Event, KeyCode, Keymap, KeymapPreset, Outcome, Theme};
use ratakit::text::SyntectHighlighter;
use ratakit::widgets::{FrameScheduler, TerminalViewport, enter_terminal_with, event_stream};
use ratakit_testing::render_stateful;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use ratatui::{Terminal, TerminalOptions};
use ratatui_crossterm::CrosstermBackend;
use tokio::task::JoinSet;

const INLINE_VIEWPORT_HEIGHT: u16 = 12;

struct NoFiles;

impl FileIndex for NoFiles {
    fn query(&self, _pattern: &str, _limit: usize) -> Vec<PathBuf> {
        Vec::new()
    }
}

type Recorder = RecordingWriter<BufWriter<File>>;

fn recorder_from_env() -> io::Result<Option<Recorder>> {
    let Some(path) = env::var_os("RATAKIT_ACP_RECORD") else {
        return Ok(None);
    };
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    RecordingWriter::new(BufWriter::new(file)).map(Some)
}

struct App {
    theme: Theme,
    highlighter: SyntectHighlighter,
    keymap: Keymap,
    layout: AgentLayoutState,
    files: NoFiles,
    recorder: Option<Recorder>,
    inline: bool,
    pending_history: Vec<Cell>,
    permission_id: Option<PermissionRequestId>,
    prompt_in_flight: bool,
    should_quit: bool,
}

impl App {
    fn new(recorder: Option<Recorder>, inline: bool) -> Self {
        let mut theme = Theme::dark();
        if env::var_os("RATAKIT_SCREEN_READER").is_some() || inline {
            theme.enable_screen_reader();
        }
        SyntectHighlighter::apply_theme(&theme);
        Self {
            theme,
            highlighter: SyntectHighlighter,
            keymap: Keymap::from_preset(KeymapPreset::Default),
            layout: AgentLayoutState::new(),
            files: NoFiles,
            recorder,
            inline,
            pending_history: Vec::new(),
            permission_id: None,
            prompt_in_flight: false,
            should_quit: false,
        }
    }

    fn apply_bridge_event(&mut self, event: BridgeEvent) -> io::Result<()> {
        match event {
            BridgeEvent::Agent(event) => {
                if let Some(recorder) = &mut self.recorder {
                    recorder.event(&event)?;
                }
                self.apply_agent_event(event);
            }
            BridgeEvent::Permission(id, request) => {
                if let Some(recorder) = &mut self.recorder {
                    recorder.permission(&request)?;
                }
                self.permission_id = Some(id);
                self.layout.present_permission(request);
            }
            BridgeEvent::Exited(result) => {
                if let Err(error) = result {
                    self.layout.apply_agent_event(AgentEvent::Error(error));
                }
                self.should_quit = true;
            }
        }
        Ok(())
    }

    fn apply_agent_event(&mut self, event: AgentEvent) {
        let turn_ended = matches!(&event, AgentEvent::TurnEnded { .. });
        self.layout.apply_agent_event(event);
        if self.inline && turn_ended {
            if let Some(mut cells) = self.layout.transcript.take_completed_cells() {
                self.pending_history.append(&mut cells);
            }
        }
    }

    fn flush_inline_history(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> io::Result<()> {
        if !self.inline || self.pending_history.is_empty() {
            return Ok(());
        }

        let mut cells = std::mem::take(&mut self.pending_history);
        let mut history = TranscriptHistory::new(&mut cells, &self.theme, &self.highlighter);
        let height = history.height(terminal.size()?.width);
        if height > 0 {
            terminal.insert_before(height, |buffer| history.render(buffer.area, buffer))?;
        }
        Ok(())
    }

    fn finish_inline_history(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> io::Result<()> {
        if self.inline && self.layout.transcript.is_busy() {
            self.apply_agent_event(AgentEvent::TurnEnded {
                reason: ratakit::agent::state::StopReason::Cancelled,
            });
        }
        if self.inline
            && let Some(mut cells) = self.layout.transcript.take_completed_cells()
        {
            self.pending_history.append(&mut cells);
        }
        self.flush_inline_history(terminal)
    }

    fn handle_input(
        &mut self,
        event: &Event,
        bridge: &AcpBridgeHandle,
        operations: &mut JoinSet<(bool, Result<(), String>)>,
    ) -> io::Result<()> {
        if matches!(event, Event::Key(key) if key.code == KeyCode::Char('q') && key.modifiers.control)
        {
            self.should_quit = true;
            return Ok(());
        }
        if let Outcome::Intent(intent) = self.layout.handle(event, &self.keymap, &self.files) {
            self.handle_intent(intent, bridge, operations)?;
        }
        Ok(())
    }

    fn handle_intent(
        &mut self,
        intent: AgentLayoutIntent,
        bridge: &AcpBridgeHandle,
        operations: &mut JoinSet<(bool, Result<(), String>)>,
    ) -> io::Result<()> {
        match intent {
            AgentLayoutIntent::Submit {
                text,
                mentions,
                attachments,
            } => {
                if self.prompt_in_flight {
                    self.notice("A prompt is already in progress.");
                    return Ok(());
                }
                if !attachments.is_empty() {
                    self.layout.apply_agent_event(AgentEvent::Notice {
                        level: Level::Warning,
                        text: "This ACP example does not send attachments.".into(),
                    });
                }
                self.start_prompt(text, mentions, bridge, operations)?;
            }
            AgentLayoutIntent::SlashCommand { name, args } => {
                if self.prompt_in_flight {
                    self.notice("A prompt is already in progress.");
                    return Ok(());
                }
                self.start_prompt(format!("/{name} {args}"), Vec::new(), bridge, operations)?;
            }
            AgentLayoutIntent::Cancel => {
                let handle = bridge.clone();
                operations.spawn(async move {
                    (
                        false,
                        handle.cancel().await.map_err(|error| error.to_string()),
                    )
                });
            }
            AgentLayoutIntent::Permission(choice) => {
                if let Some(id) = self.permission_id.take()
                    && let Err(error) = bridge.respond_permission(id, choice)
                {
                    self.layout
                        .apply_agent_event(AgentEvent::Error(error.to_string()));
                }
            }
            AgentLayoutIntent::ModelChange(choice) => {
                let handle = bridge.clone();
                operations.spawn(async move {
                    (
                        false,
                        handle
                            .set_model(&choice)
                            .await
                            .map_err(|error| error.to_string()),
                    )
                });
            }
        }
        Ok(())
    }

    fn start_prompt(
        &mut self,
        text: String,
        mentions: Vec<PathBuf>,
        bridge: &AcpBridgeHandle,
        operations: &mut JoinSet<(bool, Result<(), String>)>,
    ) -> io::Result<()> {
        if let Some(recorder) = &mut self.recorder {
            recorder.await_submit()?;
        }
        self.prompt_in_flight = true;
        let handle = bridge.clone();
        operations.spawn(async move {
            (
                true,
                handle
                    .prompt(text, &mentions)
                    .await
                    .map(|_| ())
                    .map_err(|error| error.to_string()),
            )
        });
        Ok(())
    }

    fn notice(&mut self, text: &str) {
        self.layout.apply_agent_event(AgentEvent::Notice {
            level: Level::Warning,
            text: text.into(),
        });
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let placeholder = if self.theme.screen_reader {
            "Message the ACP agent. Tab changes focus; Ctrl+M opens model selection."
        } else {
            "Message the ACP agent…"
        };
        let widget = AgentLayout::new(&self.theme, &self.highlighter, &self.files)
            .composer_placeholder(placeholder);
        frame.render_stateful_widget(widget, frame.area(), &mut self.layout);
    }
}

/// Launch an ACP-backed `AgentLayout` app. Set `RATAKIT_ACP_HEADLESS=1` for a
/// smoke render that does not launch the configured agent process.
pub async fn run(command: AgentCommand, agent_name: &str) -> io::Result<()> {
    if env::var_os("RATAKIT_ACP_HEADLESS").is_some() {
        return run_headless(agent_name);
    }

    let recorder = recorder_from_env()?;
    let working_directory = env::current_dir()?;
    let mut bridge = AcpBridge::spawn(command, &working_directory)
        .await
        .map_err(io::Error::other)?;
    bridge.new_session().await.map_err(io::Error::other)?;

    let viewport = if env::var_os("RATAKIT_INLINE").is_some() {
        TerminalViewport::Inline
    } else {
        TerminalViewport::AlternateScreen
    };
    let _session = enter_terminal_with(viewport)?;
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(stdout()),
        TerminalOptions {
            viewport: viewport.ratatui_viewport(INLINE_VIEWPORT_HEIGHT),
        },
    )?;
    terminal.clear()?;

    let mut app = App::new(recorder, viewport == TerminalViewport::Inline);
    let handle = bridge.handle();
    let mut scheduler = FrameScheduler::with_motion(app.theme.motion);
    scheduler.request();
    let mut input = event_stream();
    let mut tick = tokio::time::interval(Duration::from_millis(60));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut operations = JoinSet::new();

    while !app.should_quit {
        tokio::select! {
            event = input.next() => {
                match event {
                    Some(Ok(event)) => {
                        if let Event::Resize { width, height } = event {
                            terminal.resize(Rect::new(0, 0, width, height))?;
                        }
                        app.handle_input(&event, &handle, &mut operations)?;
                        scheduler.request();
                    }
                    Some(Err(error)) => return Err(error),
                    None => break,
                }
            }
            event = bridge.events().recv() => {
                match event {
                    Some(event) => app.apply_bridge_event(event)?,
                    None => app.should_quit = true,
                }
                app.flush_inline_history(&mut terminal)?;
                scheduler.request();
            }
            _ = scheduler.next_frame() => {
                terminal.draw(|frame| app.draw(frame))?;
            }
            _ = tick.tick() => {
                app.layout.handle(&Event::Tick(Instant::now()), &app.keymap, &app.files);
                scheduler.request();
            }
            operation = operations.join_next(), if !operations.is_empty() => {
                match operation {
                    Some(Ok((is_prompt, Ok(())))) => {
                        if is_prompt {
                            app.prompt_in_flight = false;
                        }
                    }
                    Some(Ok((is_prompt, Err(error)))) => {
                        app.layout.apply_agent_event(AgentEvent::Error(error));
                        if is_prompt {
                            app.prompt_in_flight = false;
                            app.apply_agent_event(AgentEvent::TurnEnded {
                                reason: ratakit::agent::state::StopReason::Error,
                            });
                        }
                    }
                    Some(Err(error)) => {
                        let was_prompt = app.prompt_in_flight;
                        app.layout
                            .apply_agent_event(AgentEvent::Error(error.to_string()));
                        if was_prompt {
                            app.prompt_in_flight = false;
                            app.apply_agent_event(AgentEvent::TurnEnded {
                                reason: ratakit::agent::state::StopReason::Error,
                            });
                        }
                    }
                    None => {}
                }
                app.flush_inline_history(&mut terminal)?;
                scheduler.request();
            }
        }
    }

    app.finish_inline_history(&mut terminal)?;
    Ok(())
}

fn run_headless(agent_name: &str) -> io::Result<()> {
    let inline = env::var_os("RATAKIT_INLINE").is_some();
    let mut app = App::new(None, inline);
    app.apply_agent_event(AgentEvent::AssistantDelta {
        id: MessageId::new("headless-preview"),
        text: format!("{agent_name} ACP example is ready."),
    });
    app.apply_agent_event(AgentEvent::MessageDone {
        id: MessageId::new("headless-preview"),
    });
    app.apply_agent_event(AgentEvent::TurnEnded {
        reason: ratakit::agent::state::StopReason::EndTurn,
    });
    let placeholder = if app.theme.screen_reader {
        "Message the ACP agent. Tab changes focus; Ctrl+M opens model selection."
    } else {
        "Message the ACP agent…"
    };
    let widget = AgentLayout::new(&app.theme, &app.highlighter, &app.files)
        .composer_placeholder(placeholder);
    let snapshot = render_stateful(widget, 80, 20, &mut app.layout);
    assert!(snapshot.contains("Message the ACP agent"));
    if !inline {
        assert!(snapshot.contains(agent_name));
    }
    if app.theme.screen_reader {
        assert!(snapshot.contains("composer:"));
        assert!(!snapshot.contains('╭'));
        assert!(!snapshot.contains('╰'));
    }
    if inline {
        assert!(!snapshot.contains("ACP example is ready."));
        assert!(app.pending_history.iter().any(|cell| {
            matches!(cell, Cell::Assistant(assistant) if assistant.stream.source().contains("ACP example is ready."))
        }));
    }
    println!("ok: {agent_name} ACP example headless render");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::PathBuf;

    use ratakit::agent::recording::{RecordingEntry, RecordingReader, RecordingWriter};
    use ratakit::agent::state::{
        AgentEvent, MessageId, StopReason, ToolCall, ToolCallId, ToolContent, ToolKind, ToolStatus,
    };
    use ratakit::text::DiffModel;

    #[test]
    fn recording_output_is_replayable_jsonl() {
        let mut recorder = RecordingWriter::new(Vec::new()).expect("recording header");
        recorder.await_submit().expect("write turn marker");
        recorder
            .event(&AgentEvent::AssistantDelta {
                id: MessageId::new("assistant-1"),
                text: "captured response".into(),
            })
            .expect("write assistant event");
        recorder
            .event(&AgentEvent::ToolCallStarted(ToolCall {
                id: ToolCallId::new("edit-1"),
                title: "Edit file".into(),
                kind: ToolKind::Edit,
                status: ToolStatus::Completed,
                locations: Vec::new(),
                content: vec![ToolContent::Diff {
                    path: PathBuf::from("src/lib.rs"),
                    model: DiffModel::from_texts(
                        "src/lib.rs",
                        "fn answer() {}\n",
                        "fn answer() -> u8 { 42 }\n",
                    ),
                }],
            }))
            .expect("write tool diff");
        recorder
            .event(&AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
            })
            .expect("write turn end");
        let bytes = recorder.into_inner().expect("flush recording");
        let mut reader = RecordingReader::new(Cursor::new(bytes));
        assert_eq!(
            reader.next_entry().expect("turn marker"),
            Some(RecordingEntry::AwaitSubmit)
        );
        assert!(matches!(
            reader.next_entry().expect("assistant event"),
            Some(RecordingEntry::Event(AgentEvent::AssistantDelta { text, .. }))
                if text == "captured response"
        ));
        let Some(RecordingEntry::Event(AgentEvent::ToolCallStarted(call))) =
            reader.next_entry().expect("tool event")
        else {
            panic!("recorded tool event should deserialize");
        };
        let [ToolContent::Diff { model, .. }] = call.content.as_slice() else {
            panic!("recorded diff should deserialize");
        };
        assert_eq!(
            model.source_texts(),
            Some(("fn answer() {}\n", "fn answer() -> u8 { 42 }\n"))
        );
        assert!(matches!(
            reader.next_entry().expect("turn end"),
            Some(RecordingEntry::Event(AgentEvent::TurnEnded {
                reason: StopReason::EndTurn
            }))
        ));
        assert_eq!(reader.next_entry().expect("end of recording"), None);
    }
}
