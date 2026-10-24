//! Shared terminal app for the Claude Code and OpenCode ACP examples.

use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write, stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use ratakit::acp::{AcpBridge, AcpBridgeHandle, AgentCommand, BridgeEvent, PermissionRequestId};
use ratakit::agent::state::{AgentEvent, FileIndex, Level, MessageId, PermissionRequest};
use ratakit::agent::widgets::{AgentLayout, AgentLayoutIntent, AgentLayoutState};
use ratakit::core::{Event, KeyCode, Keymap, KeymapPreset, Outcome, Theme};
use ratakit::text::SyntectHighlighter;
use ratakit::widgets::{FrameScheduler, TerminalViewport, enter_terminal_with, event_stream};
use ratakit_testing::render_stateful;
use ratatui::Terminal;
use ratatui::layout::Rect;
use ratatui_crossterm::CrosstermBackend;
use tokio::task::JoinSet;

struct NoFiles;

impl FileIndex for NoFiles {
    fn query(&self, _pattern: &str, _limit: usize) -> Vec<PathBuf> {
        Vec::new()
    }
}

struct Recorder {
    output: BufWriter<File>,
}

impl Recorder {
    fn from_env() -> io::Result<Option<Self>> {
        let Some(path) = env::var_os("RATAKIT_ACP_RECORD") else {
            return Ok(None);
        };
        Self::create(path).map(Some)
    }

    fn create(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok(Self {
            output: BufWriter::new(file),
        })
    }

    fn begin_turn(&mut self) -> io::Result<()> {
        self.output.write_all(b"{\"await_submit\":true}\n")?;
        self.output.flush()
    }

    fn event(&mut self, event: &AgentEvent) -> io::Result<()> {
        serde_json::to_writer(&mut self.output, event).map_err(io::Error::other)?;
        self.end_line()
    }

    fn permission(&mut self, request: &PermissionRequest) -> io::Result<()> {
        let line = serde_json::json!({ "permission": request });
        serde_json::to_writer(&mut self.output, &line).map_err(io::Error::other)?;
        self.end_line()
    }

    fn end_line(&mut self) -> io::Result<()> {
        self.output.write_all(b"\n")?;
        self.output.flush()
    }
}

struct App {
    theme: Theme,
    highlighter: SyntectHighlighter,
    keymap: Keymap,
    layout: AgentLayoutState,
    files: NoFiles,
    recorder: Option<Recorder>,
    permission_id: Option<PermissionRequestId>,
    prompt_in_flight: bool,
    should_quit: bool,
}

impl App {
    fn new(recorder: Option<Recorder>) -> Self {
        let theme = Theme::dark();
        SyntectHighlighter::apply_theme(&theme);
        Self {
            theme,
            highlighter: SyntectHighlighter,
            keymap: Keymap::from_preset(KeymapPreset::Default),
            layout: AgentLayoutState::new(),
            files: NoFiles,
            recorder,
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
                self.layout.apply_agent_event(event);
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
            recorder.begin_turn()?;
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
        let widget = AgentLayout::new(&self.theme, &self.highlighter, &self.files)
            .composer_placeholder("Message the ACP agent…");
        frame.render_stateful_widget(widget, frame.area(), &mut self.layout);
    }
}

/// Launch an ACP-backed `AgentLayout` app. Set `RATAKIT_ACP_HEADLESS=1` for a
/// smoke render that does not launch the configured agent process.
pub async fn run(command: AgentCommand, agent_name: &str) -> io::Result<()> {
    if env::var_os("RATAKIT_ACP_HEADLESS").is_some() {
        return run_headless(agent_name);
    }

    let recorder = Recorder::from_env()?;
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
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let mut app = App::new(recorder);
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
                        if is_prompt {
                            app.prompt_in_flight = false;
                        }
                        app.layout.apply_agent_event(AgentEvent::Error(error));
                    }
                    Some(Err(error)) => {
                        app.prompt_in_flight = false;
                        app.layout.apply_agent_event(AgentEvent::Error(error.to_string()));
                    }
                    None => {}
                }
                scheduler.request();
            }
        }
    }

    Ok(())
}

fn run_headless(agent_name: &str) -> io::Result<()> {
    let mut app = App::new(None);
    app.layout.apply_agent_event(AgentEvent::AssistantDelta {
        id: MessageId::new("headless-preview"),
        text: format!("{agent_name} ACP example is ready."),
    });
    app.layout.apply_agent_event(AgentEvent::MessageDone {
        id: MessageId::new("headless-preview"),
    });
    let widget = AgentLayout::new(&app.theme, &app.highlighter, &app.files)
        .composer_placeholder("Message the ACP agent…");
    let snapshot = render_stateful(widget, 80, 20, &mut app.layout);
    assert!(snapshot.contains("Message the ACP agent"));
    assert!(snapshot.contains(agent_name));
    println!("ok: {agent_name} ACP example headless render");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use ratakit::agent::state::{
        AgentEvent, MessageId, StopReason, ToolCall, ToolCallId, ToolContent, ToolKind, ToolStatus,
    };
    use ratakit::text::DiffModel;

    use super::Recorder;

    static NEXT_TEST_FILE: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn recording_output_is_replayable_jsonl() {
        let file_id = NEXT_TEST_FILE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "ratakit-acp-recording-{}-{file_id}.jsonl",
            std::process::id()
        ));
        let mut recorder = Recorder::create(&path).expect("create recording file");
        recorder.begin_turn().expect("write turn marker");
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
        drop(recorder);

        let lines = fs::read_to_string(&path).expect("read recording");
        let parsed = lines
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("valid JSONL"))
            .collect::<Vec<_>>();
        assert_eq!(parsed[0], serde_json::json!({ "await_submit": true }));
        assert_eq!(parsed[1]["AssistantDelta"]["text"], "captured response");
        let replayed: AgentEvent = serde_json::from_value(parsed[2].clone()).expect("replay event");
        let AgentEvent::ToolCallStarted(call) = replayed else {
            panic!("recorded tool event should deserialize");
        };
        let [ToolContent::Diff { model, .. }] = call.content.as_slice() else {
            panic!("recorded diff should deserialize");
        };
        assert_eq!(
            model.source_texts(),
            Some(("fn answer() {}\n", "fn answer() -> u8 { 42 }\n"))
        );
        assert_eq!(parsed[3]["TurnEnded"]["reason"], "EndTurn");
        fs::remove_file(path).expect("remove test recording");
    }
}
