use proc_macro::TokenStream;

mod mcp_tool;

/// `#[mcp_tool(name = "...", description = "...")]` placed **above** an axum
/// handler's `#[get]/#[post]/...` route attribute.
///
/// It re-emits the handler unchanged (so the stacked route macro still expands
/// it into the real REST route) and additionally generates, for the same
/// handler:
///   * an input-schema fn derived from the handler's data params
///     (`Path`/`Query`/`Json`/`Form` wrapper types),
///   * a dispatch fn that deserializes the MCP `args`, rebuilds the HTTP
///     request, and oneshots it through the app router via
///     [`mcp_tool::mcp_oneshot`],
///   * an `inventory::submit!` of an [`mcp_tool::McpTool`] wiring the two
///     together under the given `name` / `description`.
#[proc_macro_attribute]
pub fn mcp_tool(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // expand() lands in Task 2.3; for now classification is built/tested first.
    item
}
