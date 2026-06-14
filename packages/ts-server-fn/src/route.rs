//! Route-attribute parsing + argument classification.
//!
//! Ported from `by-macros/src/server_fn.rs` so the two crates classify
//! handler arguments identically (client / path / query / body, extractor
//! exclusion, `Form<T>` unwrap, `Option<T>` detection). Keeping this a
//! self-contained copy avoids restructuring `by-macros` into a shared lib;
//! the parse surface is small and stable.

use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::{FnArg, Ident, LitStr, Pat, PatType, Token, Type};

/// `#[get("/path", extractor: T, ...)]` — parsed attribute args.
///
/// `raw` preserves the *exact* original attribute token stream so the
/// macro can re-attach the dioxus-fullstack attribute byte-for-byte.
pub struct RouteAttr {
    pub path: LitStr,
    /// Names of server-only extractor params (e.g. `user`, `role`, `_space`).
    /// Stripped from the generated TS client.
    pub extractors: HashSet<String>,
}

impl Parse for RouteAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        let mut extractors = HashSet::new();
        // Accept three shapes used across biyard/asset and biyard/ratel:
        //   #[get("/path")]                                  — no extractors
        //   #[get("/path", name)]                            — name only
        //   #[get("/path", name: Type, name2: Type2, ...)]   — name + type
        while !input.is_empty() {
            let _ = input.parse::<Token![,]>();
            if input.is_empty() {
                break;
            }
            let name: Ident = input.parse()?;
            extractors.insert(name.to_string());
            // Optional `: <Type>` annotation — consumed and discarded.
            if input.peek(Token![:]) {
                let _: Token![:] = input.parse()?;
                let _: Type = input.parse()?;
            }
        }
        Ok(RouteAttr { path, extractors })
    }
}

/// Detect `Option<T>` syntactically by the last path segment.
pub fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Option";
        }
    }
    false
}

/// If `ty` is `Form<T>` (any path), return the inner `T`. Otherwise None.
pub fn unwrap_form_type(ty: &Type) -> Option<Type> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Form" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner.clone());
                    }
                }
            }
        }
    }
    None
}

/// Inner `T` of a `Result<T, _>`, if `ty` is one. Used to strip the error
/// half off a handler return type before mapping to a TS `Promise<T>`.
///
/// Recognizes both the std `Result<T, E>` and project-local **`Result`
/// type aliases** that fix the error half, e.g.
/// `type Result<T> = std::result::Result<T, FeatureError>` imported as
/// `use feature::types::Result as CollabResult;` and written
/// `CollabResult<T>` at the call site. Any single-generic type whose
/// last path segment ident ends in `Result` is treated as such an alias
/// and unwrapped to its first generic argument. Domain DTOs in this
/// codebase use `…Response` / `…Dto` / `…Summary` suffixes, never
/// `…Result`, so this heuristic never strips a real wire type.
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
///  - query placeholder names: `["after", "before"]`
///
/// Both `{name}` (axum 0.8+ / dioxus-fullstack) and `:name` (axum 0.7 style)
/// placeholders are accepted.
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

    // Query placeholders. Two accepted forms, both reduced to the bare
    // arg name here so they match a handler arg by identity:
    //   `?after&before`  → ["after", "before"]   (scalar keys)
    //   `?{q}`            → ["q"]                 (whole-struct query;
    //                        the renderer flattens the struct's fields)
    let query_args = query_part
        .map(|q| {
            q.split('&')
                .filter(|s| !s.is_empty())
                .map(|s| s.trim_matches(|c| c == '{' || c == '}').to_string())
                .collect()
        })
        .unwrap_or_default();

    (template, path_args, query_args)
}

/// One client-visible argument of a handler, after extractor exclusion.
pub struct ClientArg {
    pub name: Ident,
    /// The argument's type, with `Form<T>` already unwrapped to `T`.
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
    /// camelCase-free original fn name (used for TS file/function naming).
    pub fn_name: String,
    /// `"/api/posts/{}/comments"` — `{}` per path arg, in order.
    pub path_template: String,
    /// Path placeholder names, in template order.
    pub path_args: Vec<String>,
    /// All client-visible args (extractors removed), with kinds assigned.
    pub client_args: Vec<ClientArg>,
    /// Return type with `Result<T, _>` stripped to `T` (if it was a Result).
    pub ret_ty: Option<Type>,
}

/// Classify a handler's signature against a parsed route.
///
/// `ret_ty` is taken from the function's `-> T` output (None for unit).
pub fn classify(
    route: &RouteAttr,
    fn_name: &Ident,
    inputs: &syn::punctuated::Punctuated<FnArg, Token![,]>,
    output: &syn::ReturnType,
) -> HandlerMeta {
    let (path_template, path_args, query_args) = parse_path(&route.path.value());
    let path_set: HashSet<&String> = path_args.iter().collect();
    let query_set: HashSet<&String> = query_args.iter().collect();

    let typed_args: Vec<(Ident, Type)> = inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Typed(PatType { pat, ty, .. }) => {
                if let Pat::Ident(pi) = pat.as_ref() {
                    Some((pi.ident.clone(), (**ty).clone()))
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();

    let mut client_args = Vec::new();
    for (name, ty) in typed_args {
        let s = name.to_string();
        if route.extractors.contains(&s) {
            continue;
        }
        let kind = if path_set.contains(&s) {
            ArgKind::Path
        } else if query_set.contains(&s) {
            ArgKind::Query
        } else {
            ArgKind::Body
        };
        // Unwrap `Form<T>` so the TS client takes `T` directly.
        let ty = unwrap_form_type(&ty).unwrap_or(ty);
        client_args.push(ClientArg { name, ty, kind });
    }

    let ret_ty = match output {
        syn::ReturnType::Type(_, ty) => {
            let inner = result_inner(ty).cloned().unwrap_or_else(|| (**ty).clone());
            Some(inner)
        }
        syn::ReturnType::Default => None,
    };

    let _ = query_args; // query classification is carried on each ClientArg
    HandlerMeta {
        fn_name: fn_name.to_string(),
        path_template,
        path_args,
        client_args,
        ret_ty,
    }
}
