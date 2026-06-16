//! End-to-end check that the `ts-server-fn` macro expands to *compiling* pure
//! axum routes (no dioxus) and registers them via inventory. If this compiles
//! and passes, the emission contract in `ts-server-fn/src/lib.rs::emit_axum`
//! is sound against real axum + the runtime crate.

use axum::extract::{Json, Path, Query};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use ts_server_fn::{delete, get, patch, post};
use ts_server_fn_axum::{api_router, registered_routes, AsStatusCode};

#[derive(Serialize, Deserialize)]
struct Post {
    id: String,
    title: String,
}

#[derive(Serialize, Deserialize)]
struct CreatePost {
    title: String,
}

#[derive(Deserialize)]
struct ListQuery {
    #[allow(dead_code)]
    limit: Option<u32>,
}

#[derive(Serialize)]
struct ApiError {
    code: String,
}
impl AsStatusCode for ApiError {
    fn as_status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

// alias — exercises the `*Result` unwrap path on the server side too.
type ApiResult<T> = Result<T, ApiError>;

#[get("/api/posts")]
pub async fn list_posts(Query(_q): Query<ListQuery>) -> ApiResult<Vec<Post>> {
    Ok(vec![])
}

#[get("/api/posts/{id}")]
pub async fn get_post(Path(id): Path<String>) -> ApiResult<Post> {
    Ok(Post {
        id,
        title: "hi".into(),
    })
}

#[post("/api/posts")]
pub async fn create_post(Json(req): Json<CreatePost>) -> ApiResult<Post> {
    Ok(Post {
        id: "1".into(),
        title: req.title,
    })
}

#[patch("/api/posts/{id}")]
pub async fn update_post(Path(id): Path<String>, Json(req): Json<CreatePost>) -> ApiResult<Post> {
    Ok(Post { id, title: req.title })
}

#[delete("/api/posts/{id}")]
pub async fn delete_post(Path(_id): Path<String>) -> ApiResult<()> {
    Ok(())
}

#[test]
fn routes_register_and_router_builds() {
    let routes = registered_routes();
    for expected in [
        ("GET", "/api/posts"),
        ("GET", "/api/posts/{id}"),
        ("POST", "/api/posts"),
        ("PATCH", "/api/posts/{id}"),
        ("DELETE", "/api/posts/{id}"),
    ] {
        assert!(routes.contains(&expected), "missing {expected:?} in {routes:?}");
    }
    // The collected router must build without panicking.
    let _router: axum::Router = api_router();
}

// ── Runtime check: drive real requests through the collected router ──
async fn send(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: Option<&str>,
) -> (axum::http::StatusCode, String) {
    use tower::ServiceExt;
    let mut builder = axum::http::Request::builder().method(method).uri(uri);
    let req = match body {
        Some(b) => builder
            .header("content-type", "application/json")
            .body(axum::body::Body::from(b.to_owned()))
            .unwrap(),
        None => {
            let _ = &mut builder;
            builder.body(axum::body::Body::empty()).unwrap()
        }
    };
    let res = app.oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn handlers_serve_real_requests() {
    // GET with path param → 200 + JSON body from the Result→Json adapter.
    let (status, body) = send(api_router(), "GET", "/api/posts/42", None).await;
    assert_eq!(status, 200, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["id"], "42");
    assert_eq!(v["title"], "hi");

    // POST with a DTO body (D2: sent directly, decoded as Json<CreatePost>).
    let (status, body) = send(
        api_router(),
        "POST",
        "/api/posts",
        Some(r#"{"title":"hello"}"#),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["title"], "hello");

    // DELETE returning Result<(), _> → 200 with empty/`null` JSON body.
    let (status, _body) = send(api_router(), "DELETE", "/api/posts/7", None).await;
    assert_eq!(status, 200);

    // GET with query string → decodes Query<ListQuery>, returns the list.
    let (status, body) = send(api_router(), "GET", "/api/posts?limit=5", None).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body, "[]");
}
