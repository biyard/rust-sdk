//! Domain-agnostic MCP-over-axum runtime.
mod oneshot;
mod registry;
mod result;

pub use oneshot::{get_app_router, mcp_oneshot, set_app_router};
pub use registry::{all_tools, McpTool};
pub use result::{IntoMcpResult, McpResult};
