//! Generic Ratatui widgets for ratakit.
//!
//! Widgets implement [`Widget`](ratatui_core::widgets::Widget) or
//! [`StatefulWidget`](ratatui_core::widgets::StatefulWidget); interaction state
//! lives in [`ratakit_core`]. Keep state and event routing in the application,
//! render each frame, and translate [`ratakit_core::Outcome`] intents at the app
//! boundary. This crate does not own the event loop or terminal session.
//!
//! Markdown, diff, fuzzy, Crossterm, and Tokio integrations are feature-gated.
//! Check this crate's feature list when depending on it directly. The umbrella
//! `ratakit` crate provides a separate feature set; see the
//! [user guide](https://github.com/jbt95/ratakit/blob/main/docs/USER_GUIDE.md).

#![allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]

mod code_block;
#[cfg(feature = "fuzzy")]
mod completion_menu;
mod controls;
#[cfg(feature = "crossterm")]
mod crossterm;
mod dialog;
#[cfg(feature = "diff")]
mod diff_view;
#[cfg(feature = "fuzzy")]
mod fuzzy_picker;
mod key_hints;
#[cfg(feature = "markdown")]
mod markdown;
mod primitives;
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
pub use controls::{Checkbox, RadioGroup, Select, SelectOption, Switch};
pub use dialog::{ConfirmDialog, Dialog, DialogButton, Modal};
#[cfg(feature = "diff")]
pub use diff_view::{DiffMode, DiffView, DiffViewState};
#[cfg(feature = "fuzzy")]
pub use fuzzy_picker::FuzzyPicker;
/// Command-palette name for the existing fuzzy picker implementation.
#[cfg(feature = "fuzzy")]
pub type CommandPalette<'a, T> = fuzzy_picker::FuzzyPicker<'a, T>;
pub use key_hints::KeyHints;
#[cfg(feature = "markdown")]
pub use markdown::{Markdown, MarkdownState};
pub use primitives::{
    Badge, Button, ButtonSize, Card, Orientation, Progress, Separator, Skeleton, Variant,
};
pub use ratakit_core::{
    CheckboxState, DialogIntent, DialogState, FlatTreeRow, RadioGroupIntent, RadioGroupState,
    ScrollLayoutHint, ScrollState, SelectIntent, SelectState, SortDirection, SwitchState,
    TableState, TabsState, ToggleIntent, ToggleState, TreeIntent, TreePath, TreeRowSizing,
    TreeSource, TreeState,
};
#[cfg(feature = "fuzzy")]
pub use ratakit_core::{
    PickerDisplayMode, PickerIntent, PickerItem, PickerRow, PickerRowKind, PickerSecondaryAxis,
    PickerSecondaryState, PickerSection, PickerState,
};
pub use scroll_view::{ScrollView, ScrollViewContent, ScrollViewMode};
pub use spinner::Spinner;
pub use status_bar::{StatusBar, StatusSegment, StatusZone};
pub use styled_convert::{
    semantic_to_color, styled_line_to_ratatui, styled_line_to_ratatui_owned,
    styled_lines_to_ratatui, styled_span_to_ratatui, text_style_to_ratatui,
};
pub use table::{Table, TableColumn};
pub use tabs::{TabItem, Tabs};
pub use textarea::{TextArea, TextAreaCursorLayout, TextAreaState, TextIntent};
pub use toasts::Toasts;
pub use tree::Tree;
pub use virtual_list::{RowRenderer, VirtualList};
