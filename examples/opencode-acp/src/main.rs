//! Run `cargo run -p opencode-acp` with OpenCode installed.

use std::io;

use ratakit::acp::AgentCommand;

#[tokio::main]
async fn main() -> io::Result<()> {
    acp_example_app::run(AgentCommand::opencode(), "OpenCode").await
}
