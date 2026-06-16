//! `ts-server-fn` — turn an annotated handler into a **pure axum route** and
//! emit a type-safe TypeScript client at macro-expansion time.
//!
//! Each `#[get]/#[post]/#[put]/#[patch]/#[delete]` attribute:
//!
//! 1. **Server side** — rewrites the handler into idiomatic axum:
//!    - the original fn is preserved as `__<name>_impl` (signature + body
//!      unchanged — it keeps its real extractors: `User`, `Path`, `Query`,
//!      `Json`, …);
//!    - a public `async fn <name>(..) -> axum::response::Response` adapter is
//!      generated that forwards the same extractors and maps the handler's
//!      `Result<T, E>` to a response: `Ok → (200, Json(T))`,
//!      `Err → (e.as_status_code(), Json(e))`;
//!    - an `inventory::submit!` registers the route on `crate::__ts_api::
//!      ApiRoute` so the consumer's `route::api_router()` can collect every
//!      handler with no manual wiring.
//!
//! 2. **TS side** — when `TS_SERVER_FN_PACKAGE_DIR` is set at the consumer's
//!    build time, renders a typed TS client fn for the handler. Only
//!    `Path`/`Query`/`Json` arguments are reflected; every other extractor
//!    (`User`, `OptionalUser`, `Session`, …) is server-only and excluded.
//!    The request body is the `Json<T>` payload itself (no `{ "req": … }`
//!    wrapping).
//!
//! ### Consumer contract
//!
//! The expanded code references, in the **consumer** crate:
//!   - `crate::__ts_api::ApiRoute { method: &'static str, path: &'static str,
//!     register: fn(axum::Router) -> axum::Router }` + `inventory::collect!`
//!   - `axum`, `inventory` as dependencies
//!   - `AsStatusCode` in scope (method `e.as_status_code()`), which asset
//!     re-exports through `crate::*`.

mod route;
mod tsgen;
mod write_ts;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, ItemFn};

use route::RouteAttr;

/// Shared expansion for every HTTP-method attribute.
fn server_fn_impl(method: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let route = parse_macro_input!(attr as RouteAttr);
    let func = parse_macro_input!(item as ItemFn);

    // ── 1. Side effect: emit TS when generation is enabled ───────────
    if let Some(dir) = write_ts::package_dir() {
        let meta = route::classify(&route, &func.sig.ident, &func.sig.inputs, &func.sig.output);
        let rendered = tsgen::render(method, &meta);
        write_ts::write_handler(&dir, "", &rendered.fn_name_camel, &rendered.source);
    }

    // ── 2. Emit the pure-axum route ─────────────────────────────────
    expand_axum(method, &route, func).into()
}

/// Generate `__<name>_impl` (original) + `<name>` (axum adapter) +
/// `inventory::submit!` route registration.
fn expand_axum(method: &str, route: &RouteAttr, func: ItemFn) -> TokenStream2 {
    let fn_ident = func.sig.ident.clone();
    let impl_ident = format_ident!("__{}_impl", fn_ident);
    let vis = func.vis.clone();
    let attrs = func.attrs.clone();
    let asyncness = func.sig.asyncness;

    // Renamed original handler — signature/body/attrs preserved, made
    // module-private (only the adapter calls it).
    let mut impl_fn = func.clone();
    impl_fn.sig.ident = impl_ident.clone();
    impl_fn.vis = syn::Visibility::Inherited;

    // Adapter parameters: clone each typed arg's TYPE, bind to a fresh
    // `__aN` ident, and forward positionally to `__<name>_impl`. The macro
    // never interprets the extractor — axum resolves it from the type.
    let mut fwd_params: Vec<TokenStream2> = Vec::new();
    let mut fwd_args: Vec<TokenStream2> = Vec::new();
    for (i, input) in func.sig.inputs.iter().enumerate() {
        if let FnArg::Typed(pt) = input {
            let id = format_ident!("__a{}", i);
            let ty = &pt.ty;
            fwd_params.push(quote! { #id: #ty });
            fwd_args.push(quote! { #id });
        }
    }

    let call_await = if asyncness.is_some() {
        quote! { #impl_ident(#(#fwd_args),*).await }
    } else {
        quote! { #impl_ident(#(#fwd_args),*) }
    };

    // Adapter: Result<T, E> → Response. `e.as_status_code()` resolves via the
    // `AsStatusCode` trait the consumer brings into scope.
    let adapter = quote! {
        #(#attrs)*
        #vis async fn #fn_ident(#(#fwd_params),*) -> ::axum::response::Response {
            match #call_await {
                ::core::result::Result::Ok(__v) => ::axum::response::IntoResponse::into_response(
                    (::axum::http::StatusCode::OK, ::axum::Json(__v))
                ),
                ::core::result::Result::Err(__e) => {
                    let __sc = __e.as_status_code();
                    ::axum::response::IntoResponse::into_response((__sc, ::axum::Json(__e)))
                }
            }
        }
    };

    // Route registration via inventory. The register fn is a non-capturing
    // closure (path is a literal, handler is a fn item) → coerces to a fn
    // pointer.
    let axum_path = route::axum_path(&route.path.value());
    let method_ident = format_ident!("{}", method.to_lowercase());
    let method_str = method.to_string();

    let registration = quote! {
        ::inventory::submit! {
            crate::__ts_api::ApiRoute {
                method: #method_str,
                path: #axum_path,
                register: |__r: ::axum::Router| -> ::axum::Router {
                    __r.route(#axum_path, ::axum::routing::#method_ident(#fn_ident))
                },
            }
        }
    };

    quote! {
        #impl_fn
        #adapter
        #registration
    }
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

// ─── Pure render pipeline (no TokenStream) used by unit tests ─────────
#[cfg(test)]
mod snapshot_tests {
    use crate::route::{classify, RouteAttr};
    use crate::tsgen::render;
    use syn::ItemFn;

    fn render_handler(method: &str, attr_src: &str, fn_src: &str) -> String {
        let route: RouteAttr = syn::parse_str(attr_src).expect("parse route attr");
        let func: ItemFn = syn::parse_str(fn_src).expect("parse fn");
        let meta = classify(&route, &func.sig.ident, &func.sig.inputs, &func.sig.output);
        render(method, &meta).source
    }

    #[test]
    fn post_path_query_body_extractor_result() {
        // path arg + Query<T> + Json<T> + server extractor (User) + Result<T>.
        let out = render_handler(
            "POST",
            r#""/api/rooms/{room_id}/comments""#,
            r#"
            pub async fn add_comment(
                user: User,
                Path(room_id): Path<String>,
                Query(q): Query<CommentQuery>,
                Json(req): Json<AddCommentRequest>,
            ) -> Result<CommentResponse, ApiError> { unreachable!() }
            "#,
        );
        // path + query + body params; user excluded.
        assert!(
            out.contains(
                "export async function addComment(roomId: string, q: CommentQuery, req: AddCommentRequest): Promise<CommentResponse>"
            ),
            "{out}"
        );
        // body passed directly (no { req } wrapping).
        assert!(out.contains("return apiPost<CommentResponse>(__url, req);"), "{out}");
        // query serialized generically.
        assert!(out.contains("new URLSearchParams()"), "{out}");
        assert!(out.contains("Object.entries"), "{out}");
        // imports
        assert!(out.contains(r#"import type { AddCommentRequest } from "../types/AddCommentRequest";"#), "{out}");
        assert!(out.contains(r#"import type { CommentResponse } from "../types/CommentResponse";"#), "{out}");
    }

    #[test]
    fn get_path_only_no_body() {
        let out = render_handler(
            "GET",
            r#""/api/test""#,
            r#"pub async fn test_handler(user: User) -> Result<GetMeResponse, ApiError> { unreachable!() }"#,
        );
        assert!(out.contains("export async function testHandler(): Promise<GetMeResponse>"), "{out}");
        assert!(out.contains(r#"return apiGet<GetMeResponse>(__url);"#), "{out}");
        assert!(out.contains(r#"import { apiGet } from "../runtime/client";"#), "{out}");
        // Extractor `user` is stripped — no param.
        assert!(out.contains("testHandler()"), "{out}");
    }

    #[test]
    fn multi_path_params() {
        let out = render_handler(
            "DELETE",
            r#""/api/rooms/:room_id/files/:file_id""#,
            r#"pub async fn del(user: User, Path((room_id, file_id)): Path<(String, String)>) -> Result<(), E> { unreachable!() }"#,
        );
        assert!(out.contains("export async function del(roomId: string, fileId: string): Promise<void>"), "{out}");
        assert!(out.contains("encodeURIComponent(String(roomId))"), "{out}");
        assert!(out.contains("encodeURIComponent(String(fileId))"), "{out}");
    }

    #[test]
    fn vec_return_type() {
        let out = render_handler(
            "POST",
            r#""/api/bulk""#,
            r#"pub async fn bulk(Json(req): Json<BulkReq>) -> Result<Vec<Item>, ApiError> { unreachable!() }"#,
        );
        assert!(out.contains("Promise<Item[]>"), "{out}");
        assert!(out.contains("return apiPost<Item[]>(__url, req);"), "{out}");
    }
}
