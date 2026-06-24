//! End-to-end test for `#[mcp_tool]` (reduced-but-real harness).
//!
//! `#[mcp_tool]` runs outermost over a `#[get("…")]` route attribute. In
//! production that `#[get]` is `ts-server-fn`'s real route macro, which expands
//! the handler into an axum route. Here we use this crate's own crate-local,
//! no-op `#[get]` stand-in (`mcp_tool_macros::get`) so the test needs **no**
//! route-framework dev-dependency — a path dep there would drag its own
//! pre-existing `clippy -- -D warnings` debt into this crate's lint gate. The
//! matching REST route is registered by hand into the app router instead.
//!
//! This still exercises the full generated surface of `#[mcp_tool]`:
//!   1. it registered an `mcp_tool::McpTool` in inventory under the right name
//!      with a non-empty *object* input schema reflecting the `Query` type, and
//!   2. its generated `dispatch` deserializes the MCP `args`, rebuilds the
//!      request, and **oneshots through the app router** into the route, whose
//!      JSON response is echoed back in the `CallToolResult`.
//!
//! The generated MCP items are gated `#[cfg(feature = "server")]` (to match the
//! consumer's `--features server` build), so this test requires that feature.
#![cfg(feature = "server")]

use axum::extract::Query;
use mcp_tool_macros::get;
use serde::{Deserialize, Serialize};

// The `rmcp::schemars::JsonSchema` derive expands to unqualified `schemars::…`
// paths, so the `schemars` crate must be nameable at the derive site. The
// consumer crate has no direct `schemars` dep — only the `rmcp::schemars`
// re-export — so we alias it here. (Phase 3: essence must do the same in the
// module that defines the data type, or add a direct `schemars` dependency.)
use rmcp::schemars;

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
struct SearchQuery {
    query: String,
    limit: Option<u64>,
}

// ── the real REST route the tool oneshots into (registered by hand below) ──
async fn search_route(Query(q): Query<SearchQuery>) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "echoed": q.query, "limit": q.limit }))
}

// ── the handler under test: `#[mcp_tool]` over a (stubbed) route attr ──
// In production the real `#[get]` macro generates a wrapper that calls this
// handler; with the no-op stub it's never called directly (the test drives the
// generated `dispatch`, which oneshots into `search_route`), so allow dead_code.
#[allow(dead_code)]
#[mcp_tool_macros::mcp_tool(name = "search_essence", description = "search essences")]
#[get("/api/mcp/search")]
async fn search_essence_handler(Query(q): Query<SearchQuery>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "echoed": q.query, "limit": q.limit }))
}

// ── a tool whose route is intentionally absent: the oneshot gets a 404 ──
// Used to verify that the dispatch error is stringified only once (Fix 1).
#[allow(dead_code)]
#[mcp_tool_macros::mcp_tool(name = "missing_route_tool", description = "always 404")]
#[get("/api/mcp/no-such-route")]
async fn missing_route_handler(Query(q): Query<SearchQuery>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "echoed": q.query }))
}

#[tokio::test]
async fn generated_dispatch_oneshots_into_router() {
    // Register the matching REST route by hand (stands in for the real #[get]).
    let app = axum::Router::new().route("/api/mcp/search", axum::routing::get(search_route));
    mcp_tool::set_app_router(app);

    // (1) `#[mcp_tool]` registered exactly the expected tool in inventory.
    let tool = mcp_tool::all_tools()
        .find(|t| t.name == "search_essence")
        .expect("tool `search_essence` registered in inventory");
    assert_eq!(tool.description, "search essences");

    let schema = (tool.input_schema)();
    assert_eq!(schema["type"], "object", "schema is an object: {schema}");
    assert!(
        schema["properties"]["query"].is_object(),
        "schema reflects the `query` field from SearchQuery: {schema}"
    );

    // (2) generated dispatch → deserialize args → rebuild request → oneshot.
    let args = serde_json::json!({ "query": "hello", "limit": 7 });
    let result = (tool.dispatch)("hsec_test".into(), args)
        .await
        .expect("dispatch returns Ok(CallToolResult)");

    // The CallToolResult carries the route's JSON response as text content.
    let text = format!("{result:?}");
    assert!(text.contains("hello"), "route echoed the query: {text}");
    assert!(text.contains('7'), "route echoed the limit: {text}");
}

#[tokio::test]
async fn dispatch_omits_absent_optional_query_field() {
    // Idempotent across tests via OnceLock; the first to set wins.
    mcp_tool::set_app_router(
        axum::Router::new().route("/api/mcp/search", axum::routing::get(search_route)),
    );

    let tool = mcp_tool::all_tools()
        .find(|t| t.name == "search_essence")
        .expect("tool registered");

    // No `limit` → the Option field is omitted from the query string, and the
    // route still parses it as `None` (no `limit=null` reaches axum's Query).
    let args = serde_json::json!({ "query": "solo" });
    let result = (tool.dispatch)("hsec_test".into(), args)
        .await
        .expect("dispatch ok without optional field");
    let text = format!("{result:?}");
    assert!(text.contains("solo"), "echoed query: {text}");
}

#[tokio::test]
async fn dispatch_error_is_not_double_stringified() {
    // Idempotent: the router was already set by a prior test; that's fine.
    mcp_tool::set_app_router(
        axum::Router::new().route("/api/mcp/search", axum::routing::get(search_route)),
    );

    let tool = mcp_tool::all_tools()
        .find(|t| t.name == "missing_route_tool")
        .expect("tool registered");

    // The route `/api/mcp/no-such-route` is not registered → axum returns 404.
    // Before Fix 1, the error message inside ErrorData was:
    //   "-32603: mcp oneshot status 404: …"
    // (because the McpOneshotError was first wrapped into ErrorData, then
    //  IntoMcpResult called `.to_string()` on that ErrorData and wrapped it again).
    // After Fix 1, the error carries the raw McpOneshotError string:
    //   "mcp oneshot status 404: …"
    let err = (tool.dispatch)("hsec_test".into(), serde_json::json!({ "query": "x" }))
        .await
        .expect_err("dispatch should return Err for a 404 route");

    let msg: &str = &err.message;
    assert!(
        msg.contains("mcp oneshot status 404"),
        "error message should contain the raw oneshot error: {msg}"
    );
    assert!(
        !msg.contains("-32603"),
        "error message must NOT embed the JSON-RPC error code (double-stringify): {msg}"
    );
}
