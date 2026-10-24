//! Terminal setup with alternate screen as the default rendering mode.

use std::io::{self, Write};
use std::sync::OnceLock;

use crossterm::ExecutableCommand;
use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use crossterm::terminal::{
    Clear, ClearType, DisableLineWrap, EnableLineWrap, EnterAlternateScreen, LeaveAlternateScreen,
    disable_raw_mode, enable_raw_mode,
};

/// Where the TUI draws: fullscreen alternate buffer (default) or inline scrollback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalViewport {
    /// Full-screen alternate buffer (Codex-style default for ratakit).
    #[default]
    AlternateScreen,
    /// Draw in the main buffer; history stays in terminal scrollback.
    Inline,
}

/// RAII guard that restores the terminal on drop and on panic.
#[derive(Debug)]
pub struct TerminalSession {
    viewport: TerminalViewport,
    active: bool,
}

impl TerminalSession {
    /// Whether this session entered the alternate screen.
    pub fn viewport(&self) -> TerminalViewport {
        self.viewport
    }
}

/// Enter raw mode with [`TerminalViewport::AlternateScreen`] (fullscreen default).
pub fn enter_terminal() -> io::Result<TerminalSession> {
    enter_terminal_with(TerminalViewport::default())
}

/// Enter raw mode using the chosen viewport.
pub fn enter_terminal_with(viewport: TerminalViewport) -> io::Result<TerminalSession> {
    install_panic_hook();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnableBracketedPaste)?;
    stdout.execute(DisableLineWrap)?;
    if viewport == TerminalViewport::AlternateScreen {
        stdout.execute(EnterAlternateScreen)?;
        stdout.execute(Clear(ClearType::All))?;
    }
    stdout.flush()?;
    Ok(TerminalSession {
        viewport,
        active: true,
    })
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.active {
            let _ = restore_terminal(self.viewport);
            self.active = false;
        }
    }
}

fn restore_terminal(viewport: TerminalViewport) -> io::Result<()> {
    let mut stdout = io::stdout();
    if viewport == TerminalViewport::AlternateScreen {
        stdout.execute(LeaveAlternateScreen)?;
    }
    stdout.execute(EnableLineWrap)?;
    stdout.execute(DisableBracketedPaste)?;
    disable_raw_mode()?;
    stdout.flush()?;
    Ok(())
}

fn install_panic_hook() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let mut stdout = io::stdout();
            let _ = stdout.execute(LeaveAlternateScreen);
            let _ = stdout.execute(EnableLineWrap);
            let _ = stdout.execute(DisableBracketedPaste);
            let _ = stdout.flush();
            previous(info);
        }));
    });
}
