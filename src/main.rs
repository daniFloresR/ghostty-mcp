mod config;
mod data;
mod search;
mod server;
mod tools;

use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Log to stderr since stdout is the MCP protocol channel
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("Starting ghostty-mcp server");

    let server = server::GhosttyServer::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
