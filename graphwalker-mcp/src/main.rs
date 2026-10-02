use graphwalker_mcp::GraphWalkerMcp;
use rmcp::{transport::stdio, ServiceExt};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("error")),
        )
        .init();
    tracing::info!("starting GraphWalker MCP server over stdio");
    let service = GraphWalkerMcp::default().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
