//! Route-attribute parsing + argument classification.
//!
//! New (axum-handler) convention: handlers are written as idiomatic axum
//! handlers with real extractors in the signature. The macro attribute
//! carries only the method + path. Argument classification is driven by the
//! **wrapper type** of each signature argument, not by names declared in the
//! attribute:
//!
//! ```ignore
//!     #[get("/api/rooms/{room_id}/files")]
//!     pub async fn list_files(
//!         user: User,                       // extractor → skipped in TS
//!         Path(room_id): Path<String>,      // path
//!         Query(q): Query<ListFilesQuery>,  // query
//!         Json(req): Json<UpdateReq>,       // body (POST/PUT/PATCH)
//!     ) -> Result<ListFilesResponse, E> { ... }
//! ```
//!
//! Only `Path<…>`, `Query<…>`, `Json<…>` are reflected in the generated
//! TypeScript client. Every other argument type (`User`, `OptionalUser`,
//! `Session`, …) is a server-only extractor and is excluded.

use syn::parse::{Parse, ParseStream};
use syn::{FnArg, Ident, LitStr, PatType, Token, Type};

/// `#[get("/path")]` — parsed attribute args.
///
/// Only the path literal is meaningful now. For backwards tolerance during
/// the migration the parser also accepts (and ignores) any trailing
/// `, name: Type` extras left over from the old convention.
pub struct RouteAttr {
    pub path: LitStr,
}

impl Parse for RouteAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        // Tolerate & discard any legacy trailing extras (`, user: User`).
        while !input.is_empty() {
            let _ = input.parse::<Token![,]>();
            if input.is_empty() {
                break;
            }
            let _name: Ident = input.parse()?;
            if input.peek(Token![:]) {
                let _: Token![:] = input.parse()?;
                let _: Type = input.parse()?;
            }
        }
        Ok(RouteAttr { path })
    }
}

/// Inner `T` of a single-generic wrapper `W<T>` whose last segment ident is
/// `name` (e.g. `Path<String>`, `Query<Q>`, `Json<B>`). Returns the inner
/// type, or None if `ty` is not that wrapper.
fn unwrap_wrapper<'a>(ty: &'a Type, name: &str) -> Option<&'a Type> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == name {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}

/// The last path segment ident of a type, if it is a simple path type.
fn outer_ident(ty: &Type) -> Option<String> {
    if let Type::Path(tp) = ty {
        return tp.path.segments.last().map(|s| s.ident.to_string());
    }
    None
}

/// Extract the binding name from an extractor pattern. Handles both the
/// destructured form `Query(q): Query<T>` (tuple-struct pattern, single
/// inner ident) and the plain form `q: Query<T>` (ident pattern).
fn binding_name(pat: &syn::Pat, fallback: &str) -> Ident {
    match pat {
        syn::Pat::Ident(pi) => pi.ident.clone(),
        syn::Pat::TupleStruct(ts) => {
            if let Some(syn::Pat::Ident(pi)) = ts.elems.first() {
                return pi.ident.clone();
            }
            Ident::new(fallback, proc_macro2::Span::call_site())
        }
        _ => Ident::new(fallback, proc_macro2::Span::call_site()),
    }
}

/// Inner `T` of a `Result<T, _>` (or any alias whose name ends in
/// `Result`, e.g. `CollabResult<T>` / `RaResult<T>` / `Result<T>`), if
/// `ty` is one. The first generic argument is the success type.
pub fn result_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Result" || seg.ident.to_string().ends_with("Result") {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}

/// Parse `/api/posts/{id}/comments?after&before` into:
///  - format-string template `"/api/posts/{}/comments"` (path piece only)
///  - path placeholder names in order: `["id"]`
///  - query placeholder names: `["after", "before"]` (now unused for
///    classification — query is a single `Query<T>` struct — but kept for
///    completeness / path normalization).
///
/// Both `{name}` (axum 0.8+) and `:name` (axum 0.7 style) placeholders are
/// accepted.
pub fn parse_path(path: &str) -> (String, Vec<String>, Vec<String>) {
    let (path_part, query_part) = match path.find('?') {
        Some(i) => (&path[..i], Some(&path[i + 1..])),
        None => (path, None),
    };

    let mut path_args = Vec::new();
    let mut template = String::new();
    let mut chars = path_part.chars().peekable();
    let mut prev = '\0';
    while let Some(c) = chars.next() {
        if c == '{' {
            let mut name = String::new();
            while let Some(&nc) = chars.peek() {
                if nc == '}' {
                    chars.next();
                    break;
                }
                name.push(nc);
                chars.next();
            }
            path_args.push(name);
            template.push_str("{}");
            prev = '}';
        } else if c == ':' && (prev == '\0' || prev == '/') {
            let mut name = String::new();
            while let Some(&nc) = chars.peek() {
                if nc == '/' {
                    break;
                }
                name.push(nc);
                chars.next();
            }
            prev = name.chars().last().unwrap_or(':');
            path_args.push(name);
            template.push_str("{}");
        } else if c == '}' {
            template.push(c);
            prev = c;
        } else {
            template.push(c);
            prev = c;
        }
    }

    let query_args = query_part
        .map(|q| {
            q.split('&')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    (template, path_args, query_args)
}

/// Normalize a route path to axum 0.8 syntax (`:name` → `{name}`), stripping
/// any `?query` suffix (query is carried by the `Query<T>` extractor).
pub fn axum_path(path: &str) -> String {
    let path_part = match path.find('?') {
        Some(i) => &path[..i],
        None => path,
    };
    let mut out = String::new();
    let mut chars = path_part.chars().peekable();
    let mut prev = '\0';
    while let Some(c) = chars.next() {
        if c == ':' && (prev == '\0' || prev == '/') {
            out.push('{');
            while let Some(&nc) = chars.peek() {
                if nc == '/' {
                    break;
                }
                out.push(nc);
                chars.next();
            }
            out.push('}');
            prev = '}';
        } else {
            out.push(c);
            prev = c;
        }
    }
    out
}

/// One client-visible argument of a handler.
pub struct ClientArg {
    pub name: Ident,
    /// The TS-relevant type. For `Path` args this is `String`; for `Query`/
    /// `Json` it is the unwrapped inner type.
    pub ty: Type,
    pub kind: ArgKind,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ArgKind {
    Path,
    Query,
    Body,
}

/// Classified view of a handler's signature against its route.
pub struct HandlerMeta {
    pub fn_name: String,
    /// `"/api/posts/{}/comments"` — `{}` per path arg, in order.
    pub path_template: String,
    /// Path placeholder names, in template order.
    pub path_args: Vec<String>,
    /// All client-visible args (extractors removed), with kinds assigned.
    /// Order: path args (URL order), then query, then body.
    pub client_args: Vec<ClientArg>,
    /// Return type with `Result<T, _>` stripped to `T` (if it was a Result).
    pub ret_ty: Option<Type>,
}

/// Classify a handler's signature against a parsed route.
///
/// Path args come from the URL template (all typed as `string`). The single
/// `Query<T>` and single `Json<T>` arguments (if present) are pulled from the
/// signature by wrapper type. Every other argument is a server-only extractor
/// and excluded.
pub fn classify(
    route: &RouteAttr,
    fn_name: &Ident,
    inputs: &syn::punctuated::Punctuated<FnArg, Token![,]>,
    output: &syn::ReturnType,
) -> HandlerMeta {
    let (path_template, path_args, _query_names) = parse_path(&route.path.value());

    let mut client_args: Vec<ClientArg> = Vec::new();

    // Path args first, in URL order. All path params are `String` in this
    // codebase (no typed SubPartition path params yet) → TS `string`.
    let string_ty: Type = syn::parse_quote!(String);
    for name in &path_args {
        client_args.push(ClientArg {
            name: Ident::new(name, proc_macro2::Span::call_site()),
            ty: string_ty.clone(),
            kind: ArgKind::Path,
        });
    }

    // Walk the signature for Query<T> and Json<T>. Everything else is an
    // extractor (User / OptionalUser / Session / Path / …) and excluded —
    // Path is already covered by the URL above.
    let mut query: Option<ClientArg> = None;
    let mut body: Option<ClientArg> = None;
    for input in inputs.iter() {
        let FnArg::Typed(PatType { pat, ty, .. }) = input else {
            continue;
        };
        match outer_ident(ty).as_deref() {
            Some("Query") => {
                if let Some(inner) = unwrap_wrapper(ty, "Query") {
                    query = Some(ClientArg {
                        name: binding_name(pat, "query"),
                        ty: inner.clone(),
                        kind: ArgKind::Query,
                    });
                }
            }
            Some("Json") => {
                if let Some(inner) = unwrap_wrapper(ty, "Json") {
                    body = Some(ClientArg {
                        name: binding_name(pat, "body"),
                        ty: inner.clone(),
                        kind: ArgKind::Body,
                    });
                }
            }
            _ => {} // Path (handled via URL) or server-only extractor.
        }
    }
    if let Some(q) = query {
        client_args.push(q);
    }
    if let Some(b) = body {
        client_args.push(b);
    }

    let ret_ty = match output {
        syn::ReturnType::Type(_, ty) => {
            let inner = result_inner(ty).cloned().unwrap_or_else(|| (**ty).clone());
            Some(inner)
        }
        syn::ReturnType::Default => None,
    };

    HandlerMeta {
        fn_name: fn_name.to_string(),
        path_template,
        path_args,
        client_args,
        ret_ty,
    }
}
