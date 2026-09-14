use graphwalker_mcp::GraphWalkerMcp;
use rmcp::{transport::stdio, ServiceExt};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = GraphWalkerMcp.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
