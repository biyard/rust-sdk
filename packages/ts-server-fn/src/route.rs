//! Route-attribute parsing + **signature-type-based** argument classification.
//!
//! Unlike the previous attribute-name-based model (which matched arg *names*
//! against the path/query placeholders and read an extractor allow-list from
//! the attribute), this classifies purely from the **extractor wrapper type**
//! in the handler signature:
//!
//!   - `Path<T>`  → path arg(s). Names come from the binding pattern
//!                  (`Path(room_id)` → `room_id`; `Path((a, b))` → `a`, `b`).
//!   - `Query<T>` → one query arg (the struct `T` is serialized to `?k=v`).
//!   - `Json<T>`  / `Form<T>` → the request body (`T` directly — no wrapping).
//!   - anything else (`User`, `Session`, `State<_>`, …) → server extractor,
//!     stripped from the TS client.
//!
//! This is immune to the "arg renamed but route not updated" drift, and the
//! body/query/path types are read from the *actual* signature rather than a
//! parallel annotation. The trade-off (Risk #1) is that the wrapper is matched
//! by its **last path segment ident** — so `use axum::extract::Path as P` is
//! NOT recognized. The convention is: import `Path`/`Query`/`Json` unaliased.
//! Misuse is caught loudly: a `Json` body on GET/DELETE, two bodies, or a
//! path-placeholder/arg count mismatch all produce a `syn::Error`.

use syn::parse::{Parse, ParseStream};
use syn::{FnArg, GenericArgument, Ident, LitStr, Pat, PatType, PathArguments, Token, Type};

/// `#[get("/path")]` / `#[get("/path", raw)]`.
///
/// Legacy `name: Type` extractor annotations (`#[get("/p", user: User)]`) are
/// still accepted for back-compat but **ignored** — extractors are now derived
/// from the signature types, not the attribute.
pub struct RouteAttr {
    pub path: LitStr,
    /// `raw` → skip the `Result`→`Json` response adapter; the handler's own
    /// `impl IntoResponse` is registered verbatim (file download / redirect).
    pub raw: bool,
}

impl Parse for RouteAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        let mut raw = false;
        while !input.is_empty() {
            let _ = input.parse::<Token![,]>();
            if input.is_empty() {
                break;
            }
            let name: Ident = input.parse()?;
            if name == "raw" {
                raw = true;
            } else if input.peek(Token![:]) {
                // legacy `name: Type` — consume and discard.
                let _: Token![:] = input.parse()?;
                let _: Type = input.parse()?;
            }
            // bare legacy name with no type → discard (already consumed).
        }
        Ok(RouteAttr { path, raw })
    }
}

// ─────────────────────────── type helpers ───────────────────────────

/// Last path-segment ident of a type, if it is a simple path type.
fn last_ident(ty: &Type) -> Option<&Ident> {
    if let Type::Path(tp) = ty {
        return tp.path.segments.last().map(|s| &s.ident);
    }
    None
}

/// First generic argument of a path type's last segment (`Wrapper<T>` → `T`).
fn first_generic(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if let PathArguments::AngleBracketed(args) = &seg.arguments {
                for a in &args.args {
                    if let GenericArgument::Type(t) = a {
                        return Some(t);
                    }
                }
            }
        }
    }
    None
}

/// Both generics of `Wrapper<A, B>`.
fn two_generics(ty: &Type) -> (Option<&Type>, Option<&Type>) {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if let PathArguments::AngleBracketed(args) = &seg.arguments {
                let mut it = args.args.iter().filter_map(|a| match a {
                    GenericArgument::Type(t) => Some(t),
                    _ => None,
                });
                return (it.next(), it.next());
            }
        }
    }
    (None, None)
}

/// Which extractor wrapper an arg type is, by last-segment ident.
#[derive(PartialEq, Eq, Clone, Copy)]
enum Wrapper {
    Path,
    Query,
    Body, // Json or Form
    None, // server extractor
}

fn wrapper_of(ty: &Type) -> Wrapper {
    match last_ident(ty).map(|i| i.to_string()).as_deref() {
        Some("Path") => Wrapper::Path,
        Some("Query") => Wrapper::Query,
        Some("Json") | Some("Form") => Wrapper::Body,
        _ => Wrapper::None,
    }
}

/// `Result<T, E>` (or any `*Result<T, ..>` alias such as `CollabResult<T>`)
/// → `(success T, optional E)`. Aliases hide `E`, so it comes back `None`.
///
/// This is the key fix over the old `result_inner`, which only matched the
/// literal `Result` ident and therefore rendered `CollabResult<X>` as a bogus
/// named TS type `CollabResult`.
pub fn unwrap_result(ty: &Type) -> (Type, Option<Type>) {
    if let Some(id) = last_ident(ty) {
        let name = id.to_string();
        if name == "Result" || name.ends_with("Result") {
            if let Some(ok) = first_generic(ty) {
                let err = if name == "Result" {
                    two_generics(ty).1.cloned()
                } else {
                    None
                };
                return (ok.clone(), err);
            }
        }
    }
    (ty.clone(), None)
}

/// `()` empty tuple → unit (maps to TS `void`, no body parse).
fn is_unit(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(t) if t.elems.is_empty())
}

/// Whether a return type is `Result<..>` or any `*Result<..>` alias. Used by
/// the axum response adapter to decide between an `Ok/Err` match and a plain
/// `Json` wrap.
pub fn is_result_like(ty: &Type) -> bool {
    last_ident(ty)
        .map(|i| {
            let n = i.to_string();
            n == "Result" || n.ends_with("Result")
        })
        .unwrap_or(false)
}

/// Names bound by an extractor pattern: `Path(a)` → `[a]`,
/// `Path((a, b))` → `[a, b]`, `path` (plain ident) → `[path]`.
fn binding_names(pat: &Pat) -> Vec<Ident> {
    match pat {
        // `Path(inner)` / `Query(inner)` / `Json(inner)`
        Pat::TupleStruct(ts) => ts.elems.iter().flat_map(binding_names).collect(),
        Pat::Tuple(t) => t.elems.iter().flat_map(binding_names).collect(),
        Pat::Ident(pi) => vec![pi.ident.clone()],
        _ => vec![],
    }
}

// ─────────────────────────── classified meta ───────────────────────────

#[derive(PartialEq, Eq, Clone, Copy)]
#[cfg_attr(test, derive(Debug))]
pub enum ArgKind {
    Path,
    Query,
    Body,
}

#[cfg_attr(test, derive(Debug))]
pub struct ClientArg {
    pub name: String,
    pub ty: Type,
    pub kind: ArgKind,
}

#[cfg_attr(test, derive(Debug))]
pub struct HandlerMeta {
    pub fn_name: String,
    /// `"/api/posts/{}/comments"` — one `{}` per path arg, in template order.
    pub path_template: String,
    /// Path arg names in template order (used for URL interpolation).
    pub path_names: Vec<String>,
    pub client_args: Vec<ClientArg>,
    /// Success type with `Result`/alias stripped; `None` for unit (`void`).
    pub ret_ty: Option<Type>,
    /// Error type when the return is a concrete `Result<T, E>` (alias hides it).
    pub err_ty: Option<Type>,
}

/// Parse `/api/x/{id}/y?a&b` → (`"/api/x/{}/y"`, `["id"]`, `["a","b"]`).
/// Both `{name}` and `:name` placeholder styles are accepted.
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

/// Classify a handler signature against its route, type-first.
///
/// Returns a `syn::Error` on misuse so the macro can surface a clear
/// compile error (Risk #2): body on GET/DELETE, multiple bodies, or a
/// path-placeholder/arg count mismatch.
pub fn classify(
    method: &str,
    route: &RouteAttr,
    fn_name: &Ident,
    inputs: &syn::punctuated::Punctuated<FnArg, Token![,]>,
    output: &syn::ReturnType,
) -> syn::Result<HandlerMeta> {
    let (path_template, route_path_names, _route_query_names) = parse_path(&route.path.value());

    let mut path_args: Vec<ClientArg> = Vec::new();
    let mut query_args: Vec<ClientArg> = Vec::new();
    let mut body_args: Vec<ClientArg> = Vec::new();

    for input in inputs {
        let FnArg::Typed(PatType { pat, ty, .. }) = input else {
            continue; // `self` etc. — ignore
        };
        match wrapper_of(ty) {
            Wrapper::Path => {
                let names = binding_names(pat);
                let inner = first_generic(ty);
                // `Path<(A, B)>` → tuple element types; `Path<T>` → one type.
                let elem_types: Vec<Type> = match inner {
                    Some(Type::Tuple(t)) => t.elems.iter().cloned().collect(),
                    Some(t) => vec![t.clone()],
                    None => vec![],
                };
                for (i, name) in names.iter().enumerate() {
                    let ty = elem_types
                        .get(i)
                        .cloned()
                        .or_else(|| elem_types.first().cloned())
                        .unwrap_or_else(|| syn::parse_quote!(String));
                    path_args.push(ClientArg {
                        name: name.to_string(),
                        ty,
                        kind: ArgKind::Path,
                    });
                }
            }
            Wrapper::Query => {
                let name = binding_names(pat)
                    .first()
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "query".into());
                let ty = first_generic(ty).cloned().unwrap_or(ty.as_ref().clone());
                query_args.push(ClientArg {
                    name,
                    ty,
                    kind: ArgKind::Query,
                });
            }
            Wrapper::Body => {
                let name = binding_names(pat)
                    .first()
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "body".into());
                let ty = first_generic(ty).cloned().unwrap_or(ty.as_ref().clone());
                body_args.push(ClientArg {
                    name,
                    ty,
                    kind: ArgKind::Body,
                });
            }
            Wrapper::None => { /* server extractor — strip from TS */ }
        }
    }

    // ── validations (Risk #2: fail loudly, not silently) ──────────────
    // GET must not carry a body. DELETE *may* (uncommon, but axum supports it
    // and launchpad's device-unregister endpoint relies on it).
    if method == "GET" && !body_args.is_empty() {
        return Err(syn::Error::new_spanned(
            &route.path,
            "GET handler cannot take a `Json`/`Form` body",
        ));
    }
    if body_args.len() > 1 {
        return Err(syn::Error::new_spanned(
            &route.path,
            "handler has more than one `Json`/`Form` body extractor",
        ));
    }
    if path_args.len() != route_path_names.len() {
        return Err(syn::Error::new_spanned(
            &route.path,
            format!(
                "route has {} path placeholder(s) but the handler binds {} `Path` arg(s) \
                 — they must match (use `Path((a, b))` for multiple)",
                route_path_names.len(),
                path_args.len()
            ),
        ));
    }

    // Path URL interpolation uses the *route* placeholder order; bind the
    // handler's Path arg names to that order positionally.
    let path_names: Vec<String> = if path_args.iter().map(|a| &a.name).eq(route_path_names.iter()) {
        route_path_names.clone()
    } else {
        path_args.iter().map(|a| a.name.clone()).collect()
    };

    // success / error split (handles `CollabResult<T>` aliases).
    let (ret_ty, err_ty) = match output {
        syn::ReturnType::Type(_, ty) => {
            let (ok, err) = unwrap_result(ty);
            if is_unit(&ok) {
                (None, err)
            } else {
                (Some(ok), err)
            }
        }
        syn::ReturnType::Default => (None, None),
    };

    let mut client_args = path_args;
    client_args.extend(query_args);
    client_args.extend(body_args);

    Ok(HandlerMeta {
        fn_name: fn_name.to_string(),
        path_template,
        path_names,
        client_args,
        ret_ty,
        err_ty,
    })
}
