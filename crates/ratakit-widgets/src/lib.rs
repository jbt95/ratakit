//! Generic ratatui widgets for [ratakit](https://github.com/jbt95/ratakit).
//!
//! Widgets are thin [`StatefulWidget`](ratatui_core::widgets::StatefulWidget) views over state in
//! [`ratakit_core`].

mod key_hints;
mod spinner;
mod status_bar;
mod style;
mod textarea;
mod theme_util;
mod toasts;
mod wrap;

pub use key_hints::KeyHints;
pub use spinner::Spinner;
pub use status_bar::{StatusBar, StatusSegment, StatusZone};
pub use textarea::{TextArea, TextAreaState, TextIntent};
pub use toasts::Toasts;
