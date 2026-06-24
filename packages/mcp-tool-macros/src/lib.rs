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
pub fn mcp_tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    mcp_tool::expand(attr.into(), item.into()).into()
}

/// Test-only no-op stand-in for a real route attribute macro (e.g.
/// `ts-server-fn`'s `#[get]`). It strips itself and re-emits the item verbatim.
///
/// `#[mcp_tool]` runs outermost and re-emits the handler with its `#[get("…")]`
/// route attribute still attached; in production the sibling route macro then
/// expands that into a real axum route. In the end-to-end test we don't want a
/// full route framework as a dev-dependency — that would taint
/// `clippy -- -D warnings` with the dependency crate's own pre-existing lint
/// debt — so this crate-local `#[get]` resolves the stacked attribute to a
/// no-op, and the test registers the matching REST route by hand.
///
/// `#[doc(hidden)]`: not part of the public API, exists only so the integration
/// test can name a `get` route attribute (proc-macro crates can't `#[cfg(test)]`
/// a helper macro for an external test target).
#[doc(hidden)]
#[proc_macro_attribute]
pub fn get(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
