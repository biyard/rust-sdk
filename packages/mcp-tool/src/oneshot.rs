use std::sync::OnceLock;

use axum::Router;
use tower::ServiceExt;

static APP_ROUTER: OnceLock<Router> = OnceLock::new();

/// Register the app router the MCP tools re-dispatch into. Call once in `run.rs`
/// after the full router (with all REST routes) is built.
pub fn set_app_router(router: Router) {
    let _ = APP_ROUTER.set(router);
}

pub fn get_app_router() -> Router {
    APP_ROUTER
        .get()
        .expect("mcp-tool: app router not set — call set_app_router() in run.rs")
        .clone()
}

#[derive(Debug)]
pub enum McpOneshotError {
    RoutingFailed,
    Status(u16, String),
}

impl std::fmt::Display for McpOneshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RoutingFailed => write!(f, "mcp oneshot routing failed"),
            Self::Status(s, m) => write!(f, "mcp oneshot status {s}: {m}"),
        }
    }
}
impl std::error::Error for McpOneshotError {}

/// Re-dispatch an HTTP request through the app router, carrying the MCP secret
/// as `Authorization: McpSecret <secret>`. Deserializes the JSON response body.
pub async fn mcp_oneshot<T: serde::de::DeserializeOwned>(
    method: &str,
    path: &str,
    mcp_secret: &str,
    body: Option<Vec<u8>>,
) -> Result<T, McpOneshotError> {
    let router = get_app_router();
    // `#` is legal in our ids but must be percent-encoded in a URI.
    let encoded_path = path.replace('#', "%23");

    let mut builder = axum::http::Request::builder()
        .uri(format!("http://localhost{encoded_path}"))
        .method(method)
        .header("authorization", format!("McpSecret {mcp_secret}"));
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let req = builder
        .body(body.map(axum::body::Body::from).unwrap_or_else(axum::body::Body::empty))
        .map_err(|e| {
            tracing::error!("mcp oneshot: build request failed: {e}");
            McpOneshotError::RoutingFailed
        })?;

    let res = router.oneshot(req).await.map_err(|e| {
        tracing::error!("mcp oneshot: routing failed: {e}");
        McpOneshotError::RoutingFailed
    })?;
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 10 * 1024 * 1024)
        .await
        .map_err(|e| {
            tracing::error!("mcp oneshot: read body failed: {e}");
            McpOneshotError::RoutingFailed
        })?;
    if !status.is_success() {
        return Err(McpOneshotError::Status(
            status.as_u16(),
            String::from_utf8_lossy(&bytes).to_string(),
        ));
    }
    serde_json::from_slice(&bytes).map_err(|e| {
        tracing::error!("mcp oneshot: parse body failed: {e}");
        McpOneshotError::RoutingFailed
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Json, Router};

    #[tokio::test]
    async fn oneshot_routes_through_app_router_and_forwards_secret() {
        // A route that echoes back the secret header it received.
        async fn echo(headers: axum::http::HeaderMap) -> Json<serde_json::Value> {
            let auth = headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            Json(serde_json::json!({ "auth": auth }))
        }
        set_app_router(Router::new().route("/echo", get(echo)));
        let v: serde_json::Value = mcp_oneshot("GET", "/echo", "hsec_abc", None)
            .await
            .expect("oneshot ok");
        assert_eq!(v["auth"], "McpSecret hsec_abc");
    }
}
