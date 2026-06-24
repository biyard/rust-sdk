//! Integration test (plan Task 1.5): register a dummy `McpTool` by hand (no
//! macro), mount `mcp_router()` + a REST `/echo` route into one router,
//! `set_app_router`, then drive the MCP streamable-HTTP endpoint end to end.
//!
//! The server is configured stateless + `json_response` (see
//! `server::stateless_config`), so each JSON-RPC call is an independent POST
//! returning `application/json` — no session handshake / session header is
//! needed. We assert `initialize`, `tools/list`, and `tools/call` (the dummy
//! tool re-dispatches into `/echo` and echoes back the MCP secret).

use axum::{Json, Router, routing::get};
use mcp_tool::{IntoMcpResult, McpTool, mcp_router, set_app_router};
use tower::ServiceExt;

/// A REST handler the dummy tool oneshots into; reads the forwarded secret
/// from the `Authorization` header so the test can assert it round-trips.
async fn rest_echo(headers: axum::http::HeaderMap) -> Json<serde_json::Value> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    Json(serde_json::json!({ "secret_header": auth }))
}

fn dummy_schema() -> serde_json::Value {
    serde_json::json!({ "type": "object", "properties": {}, "required": [] })
}

fn dummy_dispatch(secret: String, _args: serde_json::Value) -> mcp_tool::DispatchFuture {
    Box::pin(async move {
        let v: serde_json::Value = mcp_tool::mcp_oneshot("GET", "/echo", &secret, None)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
        Ok::<_, rmcp::ErrorData>(v).into_mcp()
    })
}

inventory::submit! {
    McpTool {
        name: "dummy",
        description: "echo secret",
        input_schema: dummy_schema,
        dispatch: dummy_dispatch,
    }
}

/// POST one JSON-RPC message to `/mcp/{secret}` and return (status, parsed body).
async fn post_rpc(
    app: &Router,
    secret: &str,
    rpc: serde_json::Value,
) -> (axum::http::StatusCode, serde_json::Value) {
    let res = app
        .clone()
        .oneshot(
            axum::http::Request::post(format!("/mcp/{secret}"))
                // A real HTTP request always carries Host; the streamable-HTTP
                // service validates it (DNS-rebinding guard) before dispatch.
                .header("host", "localhost")
                .header("content-type", "application/json")
                .header("accept", "application/json, text/event-stream")
                .body(axum::body::Body::from(rpc.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 10 * 1024 * 1024)
        .await
        .unwrap();
    // json_response mode returns a single application/json JSON-RPC object.
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, body)
}

#[tokio::test]
async fn list_and_call_dummy_tool() {
    // App router holds the REST route the tool re-dispatches into + the MCP route.
    let app = Router::new()
        .route("/echo", get(rest_echo))
        .merge(mcp_router());
    set_app_router(app.clone());

    let secret = "hsec_test";

    // 1) initialize — must succeed and report our server info.
    let init = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": { "name": "t", "version": "0" }
        }
    });
    let (status, body) = post_rpc(&app, secret, init).await;
    assert!(status.is_success(), "initialize status: {status}");
    assert_eq!(body["result"]["serverInfo"]["name"], "essence-mcp");
    assert_eq!(body["result"]["protocolVersion"], "2025-03-26");

    // 2) tools/list — the inventory-registered dummy tool shows up.
    let list = serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}
    });
    let (status, body) = post_rpc(&app, secret, list).await;
    assert!(status.is_success(), "tools/list status: {status}");
    let tools = body["result"]["tools"]
        .as_array()
        .expect("tools array present");
    assert!(
        tools.iter().any(|t| t["name"] == "dummy"),
        "dummy tool listed; got: {body}"
    );

    // 3) tools/call — dummy re-dispatches into /echo and echoes the secret.
    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": { "name": "dummy", "arguments": {} }
    });
    let (status, body) = post_rpc(&app, secret, call).await;
    assert!(status.is_success(), "tools/call status: {status}");
    // CallToolResult: a single text content block carrying the JSON the tool returned.
    let text = body["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("tool result text present; got: {body}"));
    let echoed: serde_json::Value = serde_json::from_str(text).expect("tool returned JSON text");
    assert_eq!(
        echoed["secret_header"],
        format!("McpSecret {secret}"),
        "the tool re-dispatched into /echo carrying the MCP secret"
    );
}
