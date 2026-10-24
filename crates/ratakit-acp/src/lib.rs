//! Optional adapter between ACP agents and ratakit's protocol-neutral agent state.
//!
//! This crate deliberately has no ratatui dependency. Applications receive
//! [`BridgeEvent`] values and forward [`AgentEvent`](ratakit_agent::state::AgentEvent)
//! values to their chosen interface.

mod bridge;
mod mapping;

pub use bridge::{
    AcpBridge, AcpBridgeHandle, AcpError, AgentCommand, BridgeEvent, PermissionRequestId, SessionId,
};

#[cfg(test)]
mod tests;
