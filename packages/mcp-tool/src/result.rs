use rmcp::ErrorData;
use rmcp::model::{CallToolResult, Content};

pub type McpResult = Result<CallToolResult, ErrorData>;

/// Convert a handler `Result<T: Serialize, E: Display>` into an MCP tool result.
/// Success → a single JSON text content block. Error → an MCP internal error
/// carrying the (already-safe) `Display` string.
pub trait IntoMcpResult {
    fn into_mcp(self) -> McpResult;
}

impl<T, E> IntoMcpResult for Result<T, E>
where
    T: serde::Serialize,
    E: std::fmt::Display,
{
    fn into_mcp(self) -> McpResult {
        match self {
            Ok(data) => match serde_json::to_string(&data) {
                Ok(json) => Ok(CallToolResult::success(vec![Content::text(json)])),
                Err(err) => Err(ErrorData::internal_error(
                    format!("failed to serialize MCP response: {err}"),
                    None,
                )),
            },
            Err(e) => Err(ErrorData::internal_error(e.to_string(), None)),
        }
    }
}
