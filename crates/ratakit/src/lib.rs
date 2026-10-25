//! **ratakit** — reusable ratatui components plus an optional agentic-coding UI layer.
//!
//! See the [repository README](https://github.com/jbt95/ratakit) for status and roadmap.
//! The [user guide](https://github.com/jbt95/ratakit/blob/main/docs/USER_GUIDE.md) covers crate and
//! feature selection, terminal setup, accessibility, and examples. The
//! [component catalog](https://github.com/jbt95/ratakit/blob/main/docs/COMPONENTS.md) lists widget
//! contracts; the [integration cookbook](https://github.com/jbt95/ratakit/blob/main/docs/COOKBOOK.md)
//! explains event and application ownership boundaries.
//!
//! # Render a stateful widget
//!
//! Keep widget state in the application and render it into each frame. This
//! headless example uses `TestBackend`, so it does not enter a real terminal:
//!
//! ```
//! use ratakit::core::Theme;
//! use ratakit::widgets::{TextArea, TextAreaState};
//! use ratatui::backend::TestBackend;
//! use ratatui::Terminal;
//!
//! let theme = Theme::dark();
//! let mut input = TextAreaState::new();
//! let mut terminal = Terminal::new(TestBackend::new(40, 6))?;
//! terminal.draw(|frame| {
//!     frame.render_stateful_widget(
//!         TextArea::new(&theme).placeholder("Write a message…"),
//!         frame.area(),
//!         &mut input,
//!     );
//! })?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Inline terminal viewport
//!
//! Entering the main buffer is not enough by itself; configure Ratatui's
//! drawing viewport to match the terminal session:
//!
//! ```
//! use ratakit::widgets::TerminalViewport;
//! use ratatui::TerminalOptions;
//!
//! let options = TerminalOptions {
//!     viewport: TerminalViewport::Inline.ratatui_viewport(12),
//! };
//! assert_eq!(options.viewport, ratatui::Viewport::Inline(12));
//! ```

pub use ratakit_core as core;

#[cfg(feature = "widgets")]
pub use ratakit_widgets as widgets;

#[cfg(any(feature = "markdown", feature = "highlight-syntect", feature = "diff"))]
pub use ratakit_text as text;

#[cfg(feature = "agent")]
pub use ratakit_agent as agent;

#[cfg(feature = "acp")]
pub use ratakit_acp as acp;

#[cfg(feature = "crossterm")]
pub use ratakit_widgets::{
    TerminalSession, TerminalViewport, enter_terminal, enter_terminal_with, from_crossterm_event,
};

#[cfg(feature = "tokio")]
pub use ratakit_widgets::{FrameScheduler, event_stream};
