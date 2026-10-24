//! Agent-side state (no ratatui).

pub mod composer;

pub use composer::{
    Attachment, ComposerIntent, ComposerState, FileIndex, QueuedMessage, QueuedPayload,
    SlashCommand,
};
