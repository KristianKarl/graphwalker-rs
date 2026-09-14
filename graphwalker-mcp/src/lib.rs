use rmcp::{
    handler::server::wrapper::Json,
    model::{Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
use serde::Serialize;

/// MCP protocol adapter for GraphWalker.
///
/// Phase 0 intentionally exposes only a health check. GraphWalker model tools
/// are added after the shared service is extracted in later phases.
#[derive(Clone, Debug, Default)]
pub struct GraphWalkerMcp;

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct HealthResponse {
    pub status: &'static str,
    pub server_version: &'static str,
}

#[tool_router]
impl GraphWalkerMcp {
    #[tool(description = "Check that the local GraphWalker MCP server is running")]
    fn health(&self) -> Json<HealthResponse> {
        Json(HealthResponse {
            status: "ok",
            server_version: env!("CARGO_PKG_VERSION"),
        })
    }
}

#[tool_handler]
impl ServerHandler for GraphWalkerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "GraphWalker model-based testing tools. This Phase 0 server currently exposes only a health check.",
            )
    }
}
