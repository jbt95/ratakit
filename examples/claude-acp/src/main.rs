//! Run `cargo run -p claude-acp` with Node.js/npm available.

use std::io;

use ratakit::acp::AgentCommand;

#[tokio::main]
async fn main() -> io::Result<()> {
    acp_example_app::run(AgentCommand::claude_code(), "Claude Code").await
}
