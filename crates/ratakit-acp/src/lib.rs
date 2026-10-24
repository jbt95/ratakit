//! Optional adapter between ACP agents and ratakit's protocol-neutral agent state.
//!
//! This crate deliberately has no ratatui dependency. Applications receive
//! [`BridgeEvent`] values and forward [`AgentEvent`](ratakit_agent::state::AgentEvent)
//! values to their chosen interface.
//!
//! # Configure an agent command
//!
//! Constructing a command does not launch an agent. It only provides the process
//! configuration to pass to [`AcpBridge`].
//!
//! ```
//! use ratakit_acp::AgentCommand;
//! use std::path::PathBuf;
//!
//! let command = AgentCommand::opencode();
//! assert_eq!(command.program, PathBuf::from("opencode"));
//! assert_eq!(command.args, vec![String::from("acp")]);
//! ```

mod bridge;
mod mapping;

pub use bridge::{
    AcpBridge, AcpBridgeHandle, AcpError, AgentCommand, BridgeEvent, PermissionRequestId, SessionId,
};

#[cfg(test)]
mod tests;
