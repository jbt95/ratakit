//! Agent transcript, composer, and related widgets.

pub mod state;

#[cfg(feature = "widgets")]
pub mod widgets;

#[cfg(feature = "widgets")]
pub use widgets::{Composer, ComposerModelItem, ComposerWidgetState};

pub use state::{
    Attachment, ComposerIntent, ComposerState, FileIndex, QueuedMessage, QueuedPayload,
    SlashCommand,
};
