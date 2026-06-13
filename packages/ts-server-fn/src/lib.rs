//! `ts-server-fn` — wrap dioxus-fullstack server functions and emit a
//! type-safe TypeScript client at macro-expansion time.
//!
//! Each `#[get]/#[post]/#[put]/#[patch]/#[delete]` attribute:
//!
//! 1. **Re-emits the original dioxus-fullstack server fn unchanged.** The
//!    macro re-attaches `#[::dioxus::fullstack::<method>(<original attr>)]`
//!    to the function so dioxus generates the server handler + browser RPC
//!    stub exactly as before. The server keeps working 100% — this crate is
//!    a transparent wrapper.
//!
//! 2. **When `TS_SERVER_FN_PACKAGE_DIR` is set at the consumer's build
//!    time**, renders a TypeScript client function for the handler and
//!    writes it under that dir (one file per handler). The generated fn is
//!    a thin typed wrapper over a hand-written `runtime/client.ts` that owns
//!    transport, cookies, body-wrapping, and status→throw.
//!
//! Attribute syntax matches dioxus's / by-macros' (see `route.rs`):
//!
//! ```ignore
//!     #[post("/api/posts/{post_id}/comments?after", user: User)]
//!     pub async fn add_comment(
//!         post_id: FeedPartition,      // path
//!         after: Option<String>,       // query (skipped when None)
//!         req: AddCommentRequest,      // body → { "req": <value> }
//!     ) -> Result<Comment> { ... }     // → Promise<Comment>
//! ```
//!
//! Wire contract matched by the generated TS (see asset
//! `common/fullstack/server_fn.rs`):
//!   - URL = path template with `{}` substituted (each path arg
//!     `encodeURIComponent`'d) + `?k=v` query (None skipped)
//!   - POST/PUT/PATCH body = JSON object keyed by the body arg name:
//!     `{ "<argName>": <value> }`
//!   - 2xx = plain JSON of the return value; non-2xx = throw
//!   - cookies via `credentials: 'include'`

mod route;
mod tsgen;
mod write_ts;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

use route::RouteAttr;

/// Shared expansion for every HTTP-method attribute.
fn server_fn_impl(method: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    // Keep the original attribute token stream verbatim so we can re-attach
    // the dioxus-fullstack macro byte-for-byte.
    let attr2: TokenStream2 = attr.clone().into();
    let route = parse_macro_input!(attr as RouteAttr);
    let func = parse_macro_input!(item as ItemFn);

    // ── 1. Side effect: emit TS when generation is enabled ───────────
    if let Some(dir) = write_ts::package_dir() {
        let meta = route::classify(
            &route,
            &func.sig.ident,
            &func.sig.inputs,
            &func.sig.output,
        );
        let rendered = tsgen::render(method, &meta);
        // Flat layout by default ("" feature segment). Phase 2's `make
        // gen-ts` can group by module by passing a feature via a future
        // attribute key; for the spike we write directly under handlers/.
        write_ts::write_handler(&dir, "", &rendered.fn_name_camel, &rendered.source);
    }

    // ── 2. Re-emit the original server fn with the dioxus attribute ──
    let dioxus_attr = method_path(method);
    let expanded = quote! {
        #[#dioxus_attr(#attr2)]
        #func
    };
    expanded.into()
}

/// The dioxus-fullstack attribute path for a method, as a token stream
/// suitable for splicing into `#[ ... ]`.
fn method_path(method: &str) -> TokenStream2 {
    match method {
        "GET" => quote! { ::dioxus::fullstack::get },
        "POST" => quote! { ::dioxus::fullstack::post },
        "PUT" => quote! { ::dioxus::fullstack::put },
        "PATCH" => quote! { ::dioxus::fullstack::patch },
        "DELETE" => quote! { ::dioxus::fullstack::delete },
        _ => unreachable!("unsupported method {method}"),
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
//
// proc-macro crates can't host `tests/` integration tests that touch
// private modules, so the snapshot test lives as a `#[cfg(test)]` unit
// module here, exercising `classify` + `render` directly off a `syn`-parsed
// sample handler. This is the same path the macro takes at expansion time.
#[cfg(test)]
mod snapshot_tests {
    use crate::route::{classify, RouteAttr};
    use crate::tsgen::render;
    use syn::ItemFn;

    /// Parse `#[<method>(<attr>)] <fn>` pieces and run classify+render.
    fn render_handler(method: &str, attr_src: &str, fn_src: &str) -> String {
        let route: RouteAttr = syn::parse_str(attr_src).expect("parse route attr");
        let func: ItemFn = syn::parse_str(fn_src).expect("parse fn");
        let meta = classify(&route, &func.sig.ident, &func.sig.inputs, &func.sig.output);
        render(method, &meta).source
    }

    #[test]
    fn spike_path_query_body_extractor_result() {
        // Exercises: path arg + Option<query> + body + server extractor +
        // Result<T> return. This is the Phase 1 spike handler.
        let out = render_handler(
            "POST",
            r#""/api/rooms/{room_id}/comments?after", user: User"#,
            r#"
            pub async fn add_comment(
                room_id: RoomPartition,
                after: Option<String>,
                user: User,
                req: AddCommentRequest,
            ) -> Result<CommentResponse, ApiError> { unreachable!() }
            "#,
        );

        if std::env::var("BLESS").is_ok() {
            std::fs::write(
                concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/add_comment.ts"),
                &out,
            )
            .unwrap();
        }
        let expected = include_str!("../tests/fixtures/add_comment.ts");
        assert_eq!(out, expected, "\n--- generated ---\n{out}\n--- expected ---\n{expected}");
    }

    #[test]
    fn get_path_only_no_body() {
        let out = render_handler(
            "GET",
            r#""/api/test", user: User"#,
            r#"pub async fn test_handler(user: User) -> Result<GetMeResponse, ApiError> { unreachable!() }"#,
        );
        assert!(out.contains("export async function testHandler(): Promise<GetMeResponse>"));
        assert!(out.contains(r#"return apiGet<GetMeResponse>(__url);"#));
        assert!(out.contains(r#"import { apiGet } from "../runtime/client";"#));
        assert!(out.contains(r#"import type { GetMeResponse } from "../types/GetMeResponse";"#));
        // Extractor `user` is stripped — no param.
        assert!(out.contains("testHandler()"));
    }

    #[test]
    fn vec_and_optional_param_types() {
        let out = render_handler(
            "POST",
            r#""/api/bulk""#,
            r#"pub async fn bulk(req: BulkReq) -> Result<Vec<Item>, ApiError> { unreachable!() }"#,
        );
        assert!(out.contains("Promise<Item[]>"), "Vec<Item> → Item[]: {out}");
        assert!(out.contains(r#"return apiPost<Item[]>(__url, { "req": req });"#), "{out}");
    }
}
