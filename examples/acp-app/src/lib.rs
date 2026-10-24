//! Shared terminal app for the Claude Code and OpenCode ACP examples.

use std::env;
use std::io::{self, stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use ratakit::acp::{AcpBridge, AcpBridgeHandle, AgentCommand, BridgeEvent, PermissionRequestId};
use ratakit::agent::state::{AgentEvent, FileIndex, Level, MessageId};
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

struct App {
    theme: Theme,
    highlighter: SyntectHighlighter,
    keymap: Keymap,
    layout: AgentLayoutState,
    files: NoFiles,
    permission_id: Option<PermissionRequestId>,
    prompt_in_flight: bool,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        let theme = Theme::dark();
        SyntectHighlighter::apply_theme(&theme);
        Self {
            theme,
            highlighter: SyntectHighlighter,
            keymap: Keymap::from_preset(KeymapPreset::Default),
            layout: AgentLayoutState::new(),
            files: NoFiles,
            permission_id: None,
            prompt_in_flight: false,
            should_quit: false,
        }
    }

    fn apply_bridge_event(&mut self, event: BridgeEvent) {
        match event {
            BridgeEvent::Agent(event) => self.layout.apply_agent_event(event),
            BridgeEvent::Permission(id, request) => {
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
    }

    fn handle_input(
        &mut self,
        event: &Event,
        bridge: &AcpBridgeHandle,
        operations: &mut JoinSet<(bool, Result<(), String>)>,
    ) {
        if matches!(event, Event::Key(key) if key.code == KeyCode::Char('q') && key.modifiers.control)
        {
            self.should_quit = true;
            return;
        }
        if let Outcome::Intent(intent) = self.layout.handle(event, &self.keymap, &self.files) {
            self.handle_intent(intent, bridge, operations);
        }
    }

    fn handle_intent(
        &mut self,
        intent: AgentLayoutIntent,
        bridge: &AcpBridgeHandle,
        operations: &mut JoinSet<(bool, Result<(), String>)>,
    ) {
        match intent {
            AgentLayoutIntent::Submit {
                text,
                mentions,
                attachments,
            } => {
                if self.prompt_in_flight {
                    self.notice("A prompt is already in progress.");
                    return;
                }
                if !attachments.is_empty() {
                    self.layout.apply_agent_event(AgentEvent::Notice {
                        level: Level::Warning,
                        text: "This ACP example does not send attachments.".into(),
                    });
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
            }
            AgentLayoutIntent::SlashCommand { name, args } => {
                if self.prompt_in_flight {
                    self.notice("A prompt is already in progress.");
                    return;
                }
                self.prompt_in_flight = true;
                let handle = bridge.clone();
                operations.spawn(async move {
                    (
                        true,
                        handle
                            .prompt(format!("/{name} {args}"), &[])
                            .await
                            .map(|_| ())
                            .map_err(|error| error.to_string()),
                    )
                });
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

    let mut app = App::new();
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
                        app.handle_input(&event, &handle, &mut operations);
                        scheduler.request();
                    }
                    Some(Err(error)) => return Err(error),
                    None => break,
                }
            }
            event = bridge.events().recv() => {
                match event {
                    Some(event) => app.apply_bridge_event(event),
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
    let mut app = App::new();
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
