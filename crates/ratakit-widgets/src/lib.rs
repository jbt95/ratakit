//! Generic ratatui widgets for [ratakit](https://github.com/jbt95/ratakit).
//!
//! Widgets are thin [`StatefulWidget`](ratatui_core::widgets::StatefulWidget) views over state in
//! [`ratakit_core`].

#![allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]

mod code_block;
#[cfg(feature = "fuzzy")]
mod completion_menu;
#[cfg(feature = "crossterm")]
mod crossterm;
mod dialog;
#[cfg(feature = "diff")]
mod diff_view;
mod key_hints;
#[cfg(feature = "markdown")]
mod markdown;
mod scroll_view;
mod spinner;
mod status_bar;
mod style;
mod styled_convert;
mod table;
mod tabs;
mod textarea;
mod theme_util;
mod toasts;
#[cfg(feature = "tokio")]
mod tokio;
mod tree;
mod virtual_list;
mod wrap;

#[cfg(feature = "crossterm")]
pub use crossterm::{
    TerminalSession, TerminalViewport, enter_terminal, enter_terminal_with, from_crossterm_event,
};
#[cfg(feature = "tokio")]
pub use tokio::{FrameScheduler, event_stream};

pub use code_block::{CodeBlock, CodeBlockState};
#[cfg(feature = "fuzzy")]
pub use completion_menu::{CompletionMenu, CompletionMenuLayout, CompletionMenuState};
pub use dialog::{ConfirmDialog, Dialog, DialogButton, Modal};
#[cfg(feature = "diff")]
pub use diff_view::{DiffMode, DiffView, DiffViewState};
pub use key_hints::KeyHints;
#[cfg(feature = "markdown")]
pub use markdown::{Markdown, MarkdownState};
pub use ratakit_core::{
    DialogIntent, DialogState, FlatTreeRow, ScrollLayoutHint, ScrollState, SortDirection,
    TableState, TabsState, TreeIntent, TreePath, TreeRowSizing, TreeSource, TreeState,
};
pub use scroll_view::{ScrollView, ScrollViewContent, ScrollViewMode};
pub use spinner::Spinner;
pub use status_bar::{StatusBar, StatusSegment, StatusZone};
pub use styled_convert::{
    semantic_to_color, styled_line_to_ratatui, styled_lines_to_ratatui, styled_span_to_ratatui,
    text_style_to_ratatui,
};
pub use table::{Table, TableColumn};
pub use tabs::{TabItem, Tabs};
pub use textarea::{TextArea, TextAreaCursorLayout, TextAreaState, TextIntent};
pub use toasts::Toasts;
pub use tree::Tree;
pub use virtual_list::{RowRenderer, VirtualList};
