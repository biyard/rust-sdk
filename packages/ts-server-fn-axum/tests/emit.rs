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
