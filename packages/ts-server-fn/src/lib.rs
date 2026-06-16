//! `ts-server-fn` — turn `#[get]/#[post]/#[put]/#[patch]/#[delete]` handlers
//! into **pure axum routes** and emit a type-safe TypeScript client.
//!
//! This is the post-Dioxus design. Each attribute:
//!
//! 1. **Renames the original fn to `__ts_impl_<name>`** (signature + body
//!    unchanged) and emits a public `<name>` axum handler that takes the same
//!    extractors, forwards to the impl, and adapts the return:
//!    - `Result<T, E>` → `Ok` ⇒ `(200, Json(T))`, `Err` ⇒
//!      `(E.as_status_code(), Json(E))` (via the `AsStatusCode` trait from
//!      `ts-server-fn-axum`).
//!    - plain `T` ⇒ `(200, Json(T))`.
//!    - `#[get("…", raw)]` ⇒ the impl's own `IntoResponse` verbatim
//!      (downloads / redirects).
//!    The route is registered through `inventory` so `api_router()` collects
//!    every handler with no hand-written route table.
//!
//! 2. **When `TS_SERVER_FN_PACKAGE_DIR` is set**, classifies the signature by
//!    extractor *type* (`Path`/`Query`/`Json`) and writes one `.ts` client fn
//!    per handler. Request body is the DTO directly (no `{ "arg": value }`
//!    wrapping); query is the `Query<T>` struct.
//!
//! Classification is signature-type-based and validated: a `Json` body on
//! GET/DELETE, two bodies, or a path-placeholder/arg-count mismatch is a hard
//! compile error.

mod route;
mod tsgen;
mod write_ts;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, ItemFn, ReturnType};

use route::{is_result_like, RouteAttr};

/// Shared expansion for every HTTP-method attribute.
fn server_fn_impl(method: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let route = parse_macro_input!(attr as RouteAttr);
    let func = parse_macro_input!(item as ItemFn);

    // ── 1. Classify + (optionally) emit TS. Validation errors surface as
    //    compile errors regardless of whether generation is enabled. ───
    match route::classify(
        method,
        &route,
        &func.sig.ident,
        &func.sig.inputs,
        &func.sig.output,
    ) {
        Ok(meta) => {
            if let Some(dir) = write_ts::package_dir() {
                let rendered = tsgen::render(method, &meta);
                write_ts::write_handler(&dir, "", &rendered.fn_name_camel, &rendered.source);
            }
        }
        Err(e) => {
            // Keep the original fn in the output so downstream sees the symbol,
            // but attach the diagnostic.
            let err = e.to_compile_error();
            return quote! { #err #func }.into();
        }
    }

    // ── 2. Emit the pure-axum route + impl. ──────────────────────────
    match emit_axum(method, &route, func) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Build the axum route-wrapper + inventory registration, keeping the original
/// handler fn **unchanged** (same name, same `Result` return) so it stays
/// directly callable (e.g. from unit/integration tests). Only the generated
/// `__<name>_ts_route` wrapper performs the `Result`→`Json` adaptation and is
/// what `inventory` registers.
fn emit_axum(method: &str, route: &RouteAttr, func: ItemFn) -> syn::Result<TokenStream2> {
    let vis = func.vis.clone();
    let name = func.sig.ident.clone();
    let route_name = format_ident!("__{}_ts_route", name);

    // Wrapper params: clone each typed arg's TYPE, bind to `__aN`, forward to
    // the original fn (kept verbatim below).
    let mut wrapper_params: Vec<TokenStream2> = Vec::new();
    let mut forward: Vec<TokenStream2> = Vec::new();
    for (i, input) in func.sig.inputs.iter().enumerate() {
        match input {
            FnArg::Typed(pt) => {
                let id = format_ident!("__a{}", i);
                let ty = &pt.ty;
                wrapper_params.push(quote! { #id: #ty });
                forward.push(quote! { #id });
            }
            FnArg::Receiver(r) => {
                return Err(syn::Error::new_spanned(
                    r,
                    "ts-server-fn handlers must be free functions (no `self`)",
                ));
            }
        }
    }

    // Response adapter.
    let is_result = match &func.sig.output {
        ReturnType::Type(_, ty) => is_result_like(ty),
        ReturnType::Default => false,
    };
    let call = quote! { #name(#(#forward),*).await };
    let body = if route.raw {
        quote! { ::axum::response::IntoResponse::into_response(#call) }
    } else if is_result {
        quote! {
            match #call {
                ::core::result::Result::Ok(__v) => ::axum::response::IntoResponse::into_response(
                    (::axum::http::StatusCode::OK, ::axum::Json(__v))
                ),
                ::core::result::Result::Err(__e) => {
                    let __code = ::ts_server_fn_axum::AsStatusCode::as_status_code(&__e);
                    ::axum::response::IntoResponse::into_response((__code, ::axum::Json(__e)))
                }
            }
        }
    } else {
        quote! {
            ::axum::response::IntoResponse::into_response(
                (::axum::http::StatusCode::OK, ::axum::Json(#call))
            )
        }
    };

    // inventory registration. axum route path is the path part (strip query).
    let axum_path = route.path.value();
    let axum_path = axum_path.split('?').next().unwrap_or("").to_string();
    let method_router = format_ident!("{}", method.to_lowercase());
    let method_str = method;

    let expanded = quote! {
        #func

        #[doc(hidden)]
        #vis async fn #route_name(#(#wrapper_params),*) -> ::axum::response::Response {
            #body
        }

        ::ts_server_fn_axum::inventory::submit! {
            ::ts_server_fn_axum::ApiRoute {
                method: #method_str,
                path: #axum_path,
                register: |__r: ::axum::Router| __r.route(
                    #axum_path,
                    ::axum::routing::#method_router(#route_name),
                ),
            }
        }
    };
    Ok(expanded)
}

#[proc_macro_attribute]
pub fn get(attr: TokenStream, item: TokenStream) -> TokenStream {
    server_fn_impl("GET", attr, item)
}
#[proc_macro_attribute]
pub fn post(attr: TokenStream, item: TokenStream) -> TokenStream {
    server_fn_impl("POST", attr, item)
}
#[proc_macro_attribute]
pub fn put(attr: TokenStream, item: TokenStream) -> TokenStream {
    server_fn_impl("PUT", attr, item)
}
#[proc_macro_attribute]
pub fn patch(attr: TokenStream, item: TokenStream) -> TokenStream {
    server_fn_impl("PATCH", attr, item)
}
#[proc_macro_attribute]
pub fn delete(attr: TokenStream, item: TokenStream) -> TokenStream {
    server_fn_impl("DELETE", attr, item)
}

// ─── Unit tests: classify + render pipeline (no axum needed) ─────────
#[cfg(test)]
mod snapshot_tests {
    use crate::route::{classify, RouteAttr};
    use crate::tsgen::render;
    use syn::ItemFn;

    fn render_handler(method: &str, attr_src: &str, fn_src: &str) -> String {
        let route: RouteAttr = syn::parse_str(attr_src).expect("parse route attr");
        let func: ItemFn = syn::parse_str(fn_src).expect("parse fn");
        let meta = classify(method, &route, &func.sig.ident, &func.sig.inputs, &func.sig.output)
            .expect("classify ok");
        render(method, &meta).source
    }

    #[test]
    fn get_path_only() {
        let out = render_handler(
            "GET",
            r#""/api/data-room/{room_id}""#,
            r#"pub async fn get_data_room(Path(room_id): Path<String>, user: User)
                 -> CollabResult<DataRoomResponse> { unreachable!() }"#,
        );
        // alias `CollabResult<T>` unwraps to T (the old code rendered the alias!)
        assert!(out.contains("export async function getDataRoom(roomId: string): Promise<DataRoomResponse>"), "{out}");
        assert!(out.contains("`/api/data-room/${encodeURIComponent(String(roomId))}`"), "{out}");
        assert!(out.contains(r#"import type { DataRoomResponse } from "../types/DataRoomResponse";"#), "{out}");
        // server extractor `user: User` stripped
        assert!(!out.contains("user"), "{out}");
        assert!(!out.contains("CollabResult"), "alias must not leak into TS: {out}");
    }

    #[test]
    fn post_body_direct_no_wrapping() {
        let out = render_handler(
            "POST",
            r#""/api/posts""#,
            r#"pub async fn create_post(Json(req): Json<CreatePostRequest>)
                 -> Result<PostResponse, ApiError> { unreachable!() }"#,
        );
        // D2: body passed directly, not `{ "req": req }`
        assert!(out.contains("export async function createPost(req: CreatePostRequest): Promise<PostResponse>"), "{out}");
        assert!(out.contains("return apiPost<PostResponse>(`/api/posts`, req);"), "{out}");
        assert!(!out.contains(r#"{ "req""#), "must not wrap body: {out}");
        // explicit Result<T,E> → error doc
        assert!(out.contains("@throws ApiError"), "{out}");
    }

    #[test]
    fn path_tuple_and_query_struct() {
        let out = render_handler(
            "GET",
            r#""/api/rooms/{room_id}/files""#,
            r#"pub async fn list_files(Path(room_id): Path<String>, Query(q): Query<ListFilesQuery>)
                 -> Result<ListResponseFile, ApiError> { unreachable!() }"#,
        );
        assert!(out.contains("listFiles(roomId: string, q: ListFilesQuery)"), "{out}");
        // query struct forwarded as object to the runtime
        assert!(out.contains("return apiGet<ListResponseFile>(`/api/rooms/${encodeURIComponent(String(roomId))}/files`, q);"), "{out}");
    }

    #[test]
    fn delete_unit_return_is_void() {
        let out = render_handler(
            "DELETE",
            r#""/api/posts/{id}""#,
            r#"pub async fn delete_post(Path(id): Path<String>) -> Result<(), ApiError> { unreachable!() }"#,
        );
        assert!(out.contains("deletePost(id: string): Promise<void>"), "{out}");
        assert!(out.contains("return apiDelete<void>(`/api/posts/${encodeURIComponent(String(id))}`);"), "{out}");
    }

    #[test]
    fn multi_path_tuple() {
        let out = render_handler(
            "GET",
            r#""/api/rooms/{room_id}/files/{file_id}""#,
            r#"pub async fn get_file(Path((room_id, file_id)): Path<(String, String)>)
                 -> Result<FileResponse, ApiError> { unreachable!() }"#,
        );
        assert!(out.contains("getFile(roomId: string, fileId: string)"), "{out}");
        assert!(out.contains("/api/rooms/${encodeURIComponent(String(roomId))}/files/${encodeURIComponent(String(fileId))}`"), "{out}");
    }

    // ── validation errors (Risk #2) ──────────────────────────────────
    fn classify_err(method: &str, attr_src: &str, fn_src: &str) -> String {
        let route: RouteAttr = syn::parse_str(attr_src).unwrap();
        let func: ItemFn = syn::parse_str(fn_src).unwrap();
        classify(method, &route, &func.sig.ident, &func.sig.inputs, &func.sig.output)
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn err_body_on_get() {
        let e = classify_err(
            "GET",
            r#""/api/x""#,
            r#"pub async fn x(Json(req): Json<Req>) -> Result<R, E> { unreachable!() }"#,
        );
        assert!(e.contains("cannot take a `Json`"), "{e}");
    }

    #[test]
    fn err_path_count_mismatch() {
        let e = classify_err(
            "GET",
            r#""/api/x/{a}/{b}""#,
            r#"pub async fn x(Path(a): Path<String>) -> Result<R, E> { unreachable!() }"#,
        );
        assert!(e.contains("path placeholder"), "{e}");
    }
}
