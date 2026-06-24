use crate::result::McpResult;

pub type DispatchFuture = std::pin::Pin<Box<dyn std::future::Future<Output = McpResult> + Send>>;

/// A tool's dispatch entry point: given the request's MCP secret and the raw
/// JSON arguments, run the tool. Implemented by `#[mcp_tool]`-generated code.
pub type DispatchFn = fn(mcp_secret: String, args: serde_json::Value) -> DispatchFuture;

/// One registered MCP tool. `#[mcp_tool]` emits an `inventory::submit!` of this.
pub struct McpTool {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: fn() -> serde_json::Value,
    pub dispatch: DispatchFn,
}

inventory::collect!(McpTool);

/// All tools registered across the binary.
pub fn all_tools() -> impl Iterator<Item = &'static McpTool> {
    inventory::iter::<McpTool>.into_iter()
}
