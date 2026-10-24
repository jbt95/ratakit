//! Generic ratatui widgets for [ratakit](https://github.com/jbt95/ratakit).
//!
//! Widgets are thin [`StatefulWidget`](ratatui_core::widgets::StatefulWidget) views over state in
//! [`ratakit_core`].

mod key_hints;
mod spinner;
mod status_bar;
mod theme_util;
mod toasts;

pub use key_hints::KeyHints;
pub use spinner::Spinner;
pub use status_bar::{StatusBar, StatusSegment, StatusZone};
pub use toasts::Toasts;
