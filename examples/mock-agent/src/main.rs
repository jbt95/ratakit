//! Mock agent that replays `.jsonl` `AgentEvent` scripts through `AgentLayout`.
//!
//! Interactive: `cargo run -p mock-agent -- scripts/simple_chat.jsonl`
//!
//! Headless CI: `RATAKIT_MOCK_HEADLESS=1 cargo run -p mock-agent --quiet`

mod script;

use std::env;
use std::fs;
use std::io::{Write, stdout};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use ratakit::agent::state::{AgentEvent, ContentPart, MessageId};
use ratakit::agent::widgets::{AgentLayout, AgentLayoutIntent, AgentLayoutState};
use ratakit::core::{Event, Keymap, KeymapPreset, Outcome};
use ratakit::text::SyntectHighlighter;
use ratakit::widgets::{FrameScheduler, TerminalViewport, enter_terminal_with, event_stream};
use ratakit_testing::render_stateful;
use ratatui::Terminal;
use ratatui::layout::Rect;
use ratatui_crossterm::CrosstermBackend;

use script::{ReplayScript, ScriptDriver, load_script};

struct MockFiles;

impl ratakit::agent::state::FileIndex for MockFiles {
    fn query(&self, _pattern: &str, _limit: usize) -> Vec<PathBuf> {
        Vec::new()
    }
}

struct App {
    theme: ratakit::core::Theme,
    keymap: Keymap,
    layout_state: AgentLayoutState,
    driver: ScriptDriver,
    files: MockFiles,
    should_quit: bool,
}

impl App {
    fn new(script: ReplayScript) -> Self {
        let theme = ratakit::core::Theme::dark();
        SyntectHighlighter::apply_theme(&theme);
        Self {
            theme,
            keymap: Keymap::from_preset(KeymapPreset::Default),
            layout_state: AgentLayoutState::new(),
            driver: ScriptDriver::new(script),
            files: MockFiles,
            should_quit: false,
        }
    }

    fn apply_step(&mut self) {
        let Some(step) = self.driver.pop_step() else {
            return;
        };
        match step {
            script::ReplayStep::Event(ev) => self.layout_state.apply_agent_event(ev),
            script::ReplayStep::Permission(req) => self.layout_state.present_permission(req),
            script::ReplayStep::AwaitSubmit => {}
        }
    }

    fn handle_intent(&mut self, intent: AgentLayoutIntent) {
        match intent {
            AgentLayoutIntent::Submit { text, mentions, .. } => {
                let id = MessageId::new(format!("user-{}", self.driver.phase()));
                self.layout_state
                    .apply_agent_event(AgentEvent::UserMessage {
                        id,
                        content: vec![ContentPart::Text(text)],
                    });
                if !mentions.is_empty() {
                    let _ = mentions;
                }
                self.driver.on_user_submit();
                while self.driver.has_pending() {
                    self.apply_step();
                }
            }
            AgentLayoutIntent::SlashCommand { name, args } => {
                let id = MessageId::new(format!("slash-{}", self.driver.phase()));
                self.layout_state
                    .apply_agent_event(AgentEvent::UserMessage {
                        id,
                        content: vec![ContentPart::Text(format!("/{name} {args}"))],
                    });
                self.driver.on_user_submit();
                while self.driver.has_pending() {
                    self.apply_step();
                }
            }
            AgentLayoutIntent::Cancel => {
                self.layout_state.apply_agent_event(AgentEvent::TurnEnded {
                    reason: ratakit::agent::state::StopReason::Cancelled,
                });
            }
            AgentLayoutIntent::Permission(_) | AgentLayoutIntent::ModelChange(_) => {}
        }
    }

    fn handle(&mut self, ev: &Event) {
        if let Outcome::Intent(intent) = self.layout_state.handle(ev, &self.keymap, &self.files) {
            self.handle_intent(intent);
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let highlighter = SyntectHighlighter;
        let widget = AgentLayout::new(&self.theme, &highlighter, &self.files)
            .composer_placeholder("Message mock agent…");
        frame.render_stateful_widget(widget, frame.area(), &mut self.layout_state);
    }
}

fn default_script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/simple_chat.jsonl")
}

fn resolve_script_path() -> PathBuf {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Some(path) = args.first() {
        return PathBuf::from(path);
    }
    default_script_path()
}

fn run_headless(script_path: &Path) {
    let script = load_script(script_path).expect("load script");
    let mut app = App::new(script);

    const FRAMES: u32 = 24;
    for frame in 0..FRAMES {
        if frame % 4 == 0 {
            app.apply_step();
        }
        if frame == 8 {
            app.handle_intent(AgentLayoutIntent::Submit {
                text: "headless ping".into(),
                mentions: Vec::new(),
                attachments: Vec::new(),
            });
        }
        let highlighter = SyntectHighlighter;
        let widget = AgentLayout::new(&app.theme, &highlighter, &app.files);
        let snapshot = render_stateful(widget, 72, 16, &mut app.layout_state);
        assert!(
            snapshot.contains("composer")
                || snapshot.contains("Type")
                || snapshot.contains("Message"),
            "frame {frame} missing composer chrome"
        );
    }
    println!("ok: mock-agent headless {FRAMES} frames");
}

fn write_phase_snapshots(input: &Path, output: &Path) -> std::io::Result<usize> {
    let script = load_script(input).map_err(std::io::Error::other)?;
    fs::create_dir_all(output)?;
    let starts_with_wait =
        script.phases.len() > 1 && script.phases.first().is_some_and(Vec::is_empty);
    let first_phase = usize::from(starts_with_wait);
    let phase_count = script.phases.len().saturating_sub(first_phase);
    let mut app = App::new(script);

    for phase in 0..phase_count {
        if first_phase + phase > 0 {
            app.driver.on_user_submit();
        }
        while app.driver.has_pending() {
            app.apply_step();
        }
        let widget = AgentLayout::new(&app.theme, &SyntectHighlighter, &app.files);
        let rendered = render_stateful(widget, 80, 24, &mut app.layout_state);
        let snapshot = rendered
            .split_once("--- styles ---\n")
            .map_or(rendered.as_str(), |(content, _)| content);
        let snapshot_path = output.join(format!("phase-{:02}.snap", phase + 1));
        let mut snapshot_file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(snapshot_path)?;
        snapshot_file.write_all(snapshot.as_bytes())?;
    }
    Ok(phase_count)
}

fn run_snapshot_converter(args: &[String]) -> std::io::Result<bool> {
    if args
        .first()
        .is_none_or(|argument| argument != "--snapshots")
    {
        return Ok(false);
    }
    if args.len() != 3 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "usage: mock-agent --snapshots <recording.jsonl> <output-directory>",
        ));
    }
    let count = write_phase_snapshots(Path::new(&args[1]), Path::new(&args[2]))?;
    println!("wrote {count} phase snapshots to {}", args[2]);
    Ok(true)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if run_snapshot_converter(&args)? {
        return Ok(());
    }
    let script_path = resolve_script_path();
    if env::var("RATAKIT_MOCK_HEADLESS").is_ok() {
        run_headless(&script_path);
        return Ok(());
    }

    let script = load_script(&script_path).map_err(std::io::Error::other)?;
    let viewport = if env::var("RATAKIT_INLINE").is_ok() {
        TerminalViewport::Inline
    } else {
        TerminalViewport::AlternateScreen
    };
    let _session = enter_terminal_with(viewport)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let mut app = App::new(script);
    let mut scheduler = FrameScheduler::with_motion(app.theme.motion);
    scheduler.request();

    let mut events = event_stream();
    let mut tick = tokio::time::interval(Duration::from_millis(60));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut replay = tokio::time::interval(Duration::from_millis(120));
    replay.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    while !app.should_quit {
        tokio::select! {
            maybe = events.next() => {
                match maybe {
                    Some(Ok(ev)) => {
                        if let Event::Resize { width, height } = ev {
                            terminal.resize(Rect::new(0, 0, width, height))?;
                        }
                        if matches!(&ev, Event::Key(k) if k.code == ratakit::core::KeyCode::Char('q')
                            && k.modifiers.control) {
                            app.should_quit = true;
                        }
                        app.handle(&ev);
                        scheduler.request();
                    }
                    Some(Err(err)) => return Err(err),
                    None => break,
                }
            }
            _ = scheduler.next_frame() => {
                terminal.draw(|f| app.draw(f))?;
            }
            _ = tick.tick() => {
                app.handle(&Event::Tick(Instant::now()));
                scheduler.request();
            }
            _ = replay.tick() => {
                if app.driver.has_pending() {
                    app.apply_step();
                    scheduler.request();
                }
            }
        }
    }

    Ok(())
}
