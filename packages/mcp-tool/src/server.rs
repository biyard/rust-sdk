//! Inventory-driven rmcp `ServerHandler` + the `ANY /mcp/{secret}` streamable-HTTP router.
//!
//! A single [`GenericMcpServer`] implements [`rmcp::ServerHandler`] **manually**
//! (no `#[tool_router]`/`#[rmcp::tool]` macros): `list_tools` enumerates the
//! inventory-registered [`crate::McpTool`]s, and `call_tool` finds a tool by
//! name and runs its `dispatch` fn. One streamable-HTTP service is created per
//! secret and cached with a TTL + size bound.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Implementation, ListToolsResult, PaginatedRequestParams,
    ProtocolVersion, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use rmcp::{ErrorData, RoleServer, ServerHandler};
use tokio::sync::RwLock;

use crate::registry::all_tools;

/// A generic MCP server bound to one request's `mcp_secret`. Every tool it
/// exposes re-dispatches into the app router carrying that secret.
#[derive(Clone)]
struct GenericMcpServer {
    mcp_secret: String,
}

impl ServerHandler for GenericMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::V_2025_03_26)
            .with_server_info(Implementation::new(
                "essence-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Essence MCP. Semantically search a connected Essence House's knowledge.",
            )
    }

    async fn list_tools(
        &self,
        _req: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let tools: Vec<Tool> = all_tools()
            .map(|t| {
                let schema = (t.input_schema)();
                let obj = schema.as_object().cloned().unwrap_or_default();
                Tool::new(t.name, t.description, Arc::new(obj))
            })
            .collect();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        req: CallToolRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let tool = all_tools().find(|t| t.name == req.name).ok_or_else(|| {
            ErrorData::invalid_params(format!("unknown tool: {}", req.name), None)
        })?;
        let args = serde_json::Value::Object(req.arguments.unwrap_or_default());
        (tool.dispatch)(self.mcp_secret.clone(), args).await
    }
}

type McpService = StreamableHttpService<GenericMcpServer, LocalSessionManager>;

/// Maximum number of cached services, to bound memory.
const MAX_CACHE: usize = 1000;
/// Cached-service TTL in seconds (1 hour).
const TTL_SECS: i64 = 3600;

#[derive(Clone)]
struct Cached {
    service: McpService,
    created_at: i64,
}

static SERVICES: std::sync::LazyLock<RwLock<HashMap<String, Cached>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

/// Build the stateless streamable-HTTP config for an in-process MCP service.
///
/// - `stateful_mode: false` — each request is independent; no session is kept
///   server-side (the secret in the path is the per-request identity).
/// - `json_response: true` — return `application/json` directly instead of SSE
///   framing, since these are simple request/response tool calls re-dispatched
///   in-process.
/// - `allowed_hosts: []` — disable `Host`-header (DNS-rebinding) validation.
///   This service is never exposed directly; it is reached only via the app's
///   own `/mcp/{secret}` route, where the unguessable secret is the auth gate,
///   so the loopback-only default would just reject legitimate in-process and
///   reverse-proxied requests that carry an external `Host`.
fn stateless_config() -> StreamableHttpServerConfig {
    // `StreamableHttpServerConfig` is `#[non_exhaustive]`, so build from the
    // default and override via its `with_*` builders rather than a literal.
    StreamableHttpServerConfig::default()
        .with_sse_keep_alive(Some(std::time::Duration::from_secs(15)))
        .with_stateful_mode(false)
        .with_json_response(true)
        .disable_allowed_hosts()
}

async fn get_or_create_service(secret: &str) -> McpService {
    let now = chrono::Utc::now().timestamp();

    // Fast path: a non-expired cached service for this secret.
    {
        let cache = SERVICES.read().await;
        if let Some(e) = cache.get(secret)
            && now - e.created_at < TTL_SECS
        {
            return e.service.clone();
        }
    }

    let secret_owned = secret.to_string();
    let service = StreamableHttpService::new(
        move || {
            Ok(GenericMcpServer {
                mcp_secret: secret_owned.clone(),
            })
        },
        Arc::new(LocalSessionManager::default()),
        stateless_config(),
    );

    let mut cache = SERVICES.write().await;

    // Evict expired entries.
    cache.retain(|_, e| now - e.created_at < TTL_SECS);

    // Evict oldest entries while at capacity.
    while cache.len() >= MAX_CACHE {
        if let Some(k) = cache
            .iter()
            .min_by_key(|(_, e)| e.created_at)
            .map(|(k, _)| k.clone())
        {
            cache.remove(&k);
        } else {
            break;
        }
    }

    cache.insert(
        secret.to_string(),
        Cached {
            service: service.clone(),
            created_at: now,
        },
    );
    service
}

/// Invalidate a cached service when its secret is rotated/revoked. Idempotent.
pub async fn invalidate_service(secret: &str) {
    SERVICES.write().await.remove(secret);
}

/// `ANY /mcp/{secret}` — the MCP streamable-HTTP endpoint.
pub fn mcp_router() -> Router {
    Router::new().route("/mcp/{secret}", axum::routing::any(handle))
}

async fn handle(
    axum::extract::Path(secret): axum::extract::Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> axum::response::Response {
    let service = get_or_create_service(&secret).await;
    let resp = service.handle(request).await;
    let (parts, body) = resp.into_parts();
    axum::response::Response::from_parts(parts, axum::body::Body::new(body))
}
