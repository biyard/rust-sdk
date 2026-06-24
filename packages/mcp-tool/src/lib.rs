//! Domain-agnostic MCP-over-axum runtime.
mod oneshot;
mod registry;
mod result;
mod server;

pub use oneshot::{McpOneshotError, get_app_router, mcp_oneshot, set_app_router};
pub use registry::{DispatchFn, DispatchFuture, McpTool, all_tools};
pub use result::{IntoMcpResult, McpResult};
pub use server::{invalidate_service, mcp_router};
