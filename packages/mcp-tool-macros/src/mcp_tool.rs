use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, FnArg, GenericArgument, ItemFn, LitStr, PathArguments, Token, Type,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

// ───────────────────────── classification ─────────────────────────

/// How a handler parameter is treated by `#[mcp_tool]`.
///
/// `Path`/`Query`/`Json`/`Form` are **data params**: their inner type `T` is
/// reflected into the tool's input schema and rebuilt from the MCP `args` at
/// dispatch time. Every other parameter (custom extractors such as `McpHouse`,
/// `User`, `State<…>`) is an [`ParamKind::Extractor`] and excluded.
pub(crate) enum ParamKind {
    Path(Type),
    Query(Type),
    Json(Type),
    Form(Type),
    Extractor,
}

/// The single generic argument type of a path type, e.g. `Query<T>` → `T`.
fn inner_generic(ty: &Type) -> Option<Type> {
    let Type::Path(tp) = ty else {
        return None;
    };
    let seg = tp.path.segments.last()?;
    let PathArguments::AngleBracketed(ab) = &seg.arguments else {
        return None;
    };
    ab.args.iter().find_map(|a| match a {
        GenericArgument::Type(t) => Some(t.clone()),
        _ => None,
    })
}

/// Classify a handler parameter by its outermost wrapper type.
pub(crate) fn classify(arg: &FnArg) -> ParamKind {
    let FnArg::Typed(pt) = arg else {
        return ParamKind::Extractor;
    };
    let Type::Path(tp) = &*pt.ty else {
        return ParamKind::Extractor;
    };
    let Some(seg) = tp.path.segments.last() else {
        return ParamKind::Extractor;
    };
    let name = seg.ident.to_string();
    let inner = inner_generic(&pt.ty);
    match (name.as_str(), inner) {
        ("Path", Some(t)) => ParamKind::Path(t),
        ("Query", Some(t)) => ParamKind::Query(t),
        ("Json", Some(t)) => ParamKind::Json(t),
        ("Form", Some(t)) => ParamKind::Form(t),
        _ => ParamKind::Extractor,
    }
}

// ───────────────────────── attr args ─────────────────────────

/// Parsed attributes from `#[mcp_tool(name = "...", description = "...")]`.
struct Args {
    name: LitStr,
    description: LitStr,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut description = None;
        let pairs = Punctuated::<syn::MetaNameValue, Token![,]>::parse_terminated(input)?;
        for p in pairs {
            let key = p
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default();
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = p.value
            {
                match key.as_str() {
                    "name" => name = Some(s),
                    "description" => description = Some(s),
                    _ => {}
                }
            }
        }
        Ok(Args {
            name: name.ok_or_else(|| input.error("#[mcp_tool] is missing `name = \"...\"`"))?,
            description: description
                .ok_or_else(|| input.error("#[mcp_tool] is missing `description = \"...\"`"))?,
        })
    }
}

// ───────────────────────── route attr parsing (ported from by-macros) ─────────────────────────

/// The HTTP method + route path extracted from a stacked `#[get]/#[post]/...`.
struct RouteInfo {
    method: String,
    path: String,
    /// Named `{param}` (or `:param`) segments, in path order.
    path_params: Vec<String>,
}

/// Find and parse the route attribute on the handler. Ported from
/// `by-macros::mcp_tool::parse_route_attr` (framework-agnostic string ops),
/// returning `None` (instead of panicking) when no route attr is present so the
/// caller can emit a `compile_error!`.
fn find_route(func: &ItemFn) -> Option<RouteInfo> {
    func.attrs.iter().find_map(parse_route_attr)
}

fn parse_route_attr(attr: &Attribute) -> Option<RouteInfo> {
    let path_ident = attr.path().get_ident()?;
    let name = path_ident.to_string();
    if !matches!(name.as_str(), "post" | "get" | "put" | "patch" | "delete") {
        return None;
    }
    let method = name.to_uppercase();

    let syn::Meta::List(meta_list) = &attr.meta else {
        return None;
    };
    let tokens = meta_list.tokens.clone();
    // First token of the route attr is the string-literal path; ignore the rest.
    let parser = |input: ParseStream| -> syn::Result<String> {
        let path: LitStr = input.parse()?;
        while !input.is_empty() {
            let _ = input.parse::<proc_macro2::TokenTree>();
        }
        Ok(path.value())
    };
    let route_path = syn::parse::Parser::parse2(parser, tokens).ok()?;
    let path_params = extract_route_params(&route_path);
    Some(RouteInfo {
        method,
        path: route_path,
        path_params,
    })
}

/// Extract the named `{param}` / `:param` path segments. Ported (and trimmed to
/// path params only — query params are reconstructed from the wrapper type's
/// fields, not the route string) from `by-macros::mcp_tool::extract_route_params`.
fn extract_route_params(route: &str) -> Vec<String> {
    let path_part = match route.find('?') {
        Some(idx) => &route[..idx],
        None => route,
    };
    let mut path_params = Vec::new();
    for segment in path_part.split('/') {
        if segment.starts_with('{') && segment.ends_with('}') {
            path_params.push(segment[1..segment.len() - 1].to_string());
        } else if let Some(stripped) = segment.strip_prefix(':') {
            path_params.push(stripped.to_string());
        }
    }
    path_params
}

/// Build a `format!`-style path template, replacing each named param segment
/// with `{}`. Ported verbatim from `by-macros::mcp_tool::build_format_path`.
fn build_format_path(route: &str) -> String {
    let path_part = match route.find('?') {
        Some(idx) => &route[..idx],
        None => route,
    };
    let segments: Vec<&str> = path_part.split('/').filter(|s| !s.is_empty()).collect();
    let mut result = String::new();
    for segment in segments {
        result.push('/');
        if (segment.starts_with('{') && segment.ends_with('}')) || segment.starts_with(':') {
            result.push_str("{}");
        } else {
            result.push_str(segment);
        }
    }
    result
}

// ───────────────────────── expand ─────────────────────────

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = match syn::parse2::<Args>(attr) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error(),
    };
    let func = match syn::parse2::<ItemFn>(item.clone()) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error(),
    };

    // 1. Locate the stacked route attribute (#[get("…")]) → method + path.
    let route = match find_route(&func) {
        Some(r) => r,
        None => {
            return syn::Error::new_spanned(
                &func.sig.ident,
                "#[mcp_tool] requires a route attribute (#[get]/#[post]/#[put]/#[patch]/#[delete]) directly below it",
            )
            .to_compile_error();
        }
    };

    // 2. Classify params into data params (by wrapper type) vs extractors (dropped).
    let mut path_types: Vec<Type> = Vec::new();
    let mut query_types: Vec<Type> = Vec::new();
    let mut body_types: Vec<Type> = Vec::new();
    for arg in &func.sig.inputs {
        match classify(arg) {
            ParamKind::Path(t) => path_types.push(t),
            ParamKind::Query(t) => query_types.push(t),
            ParamKind::Json(t) | ParamKind::Form(t) => body_types.push(t),
            ParamKind::Extractor => {}
        }
    }

    // YAGNI: a single JSON/Form body is the contract (one request shape).
    if body_types.len() > 1 {
        return syn::Error::new_spanned(
            &func.sig.ident,
            "#[mcp_tool] supports at most one Json/Form body param",
        )
        .to_compile_error();
    }

    let fname = &func.sig.ident;
    let schema_fn = format_ident!("__{}_input_schema", fname);
    let dispatch_fn = format_ident!("__{}_dispatch", fname);
    let name = &args.name;
    let description = &args.description;
    let method = &route.method;
    let format_path = build_format_path(&route.path);

    // Ordered list of every data type, used to merge their schemas into one
    // object (Path + Query + body fields all live in one flat MCP args object).
    let all_data_types: Vec<&Type> = path_types
        .iter()
        .chain(query_types.iter())
        .chain(body_types.iter())
        .collect();

    // 3. input schema — merge each data type's schemars schema into one object.
    let schema_body = build_schema_body(&all_data_types);

    // 4. dispatch — deserialize args into each data type, rebuild path/query/body.
    let rebuild = build_rebuild(
        &route.path_params,
        &format_path,
        &path_types,
        &query_types,
        &body_types,
    );

    quote! {
        #func

        #[cfg(feature = "server")]
        fn #schema_fn() -> serde_json::Value {
            #schema_body
        }

        #[cfg(feature = "server")]
        fn #dispatch_fn(
            mcp_secret: String,
            args: serde_json::Value,
        ) -> mcp_tool::DispatchFuture {
            Box::pin(async move {
                use mcp_tool::IntoMcpResult;
                #rebuild
                let __res: ::std::result::Result<serde_json::Value, mcp_tool::McpOneshotError> =
                    mcp_tool::mcp_oneshot(#method, &__path, &mcp_secret, __body).await;
                __res
                    .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
                    .into_mcp()
            })
        }

        #[cfg(feature = "server")]
        inventory::submit! {
            mcp_tool::McpTool {
                name: #name,
                description: #description,
                input_schema: #schema_fn,
                dispatch: #dispatch_fn,
            }
        }
    }
}

/// Generate the body of the `__…_input_schema` fn.
///
/// * 0 data types → an empty object schema.
/// * 1 data type  → that type's `schema_for!` serialized to a `Value`.
/// * N data types → start from the first, then fold each other type's
///   `properties` / `required` into the same object (one flat MCP args object).
fn build_schema_body(types: &[&Type]) -> TokenStream {
    match types {
        [] => quote! { serde_json::json!({ "type": "object", "properties": {} }) },
        [only] => quote! {
            serde_json::to_value(rmcp::schemars::schema_for!(#only))
                .unwrap_or_else(|_| serde_json::json!({ "type": "object" }))
        },
        [first, rest @ ..] => {
            let merges = rest.iter().map(|t| {
                quote! {
                    if let Ok(__s) = serde_json::to_value(rmcp::schemars::schema_for!(#t)) {
                        if let Some(__props) = __s.get("properties").and_then(|v| v.as_object()) {
                            __merged_props.extend(__props.clone());
                        }
                        if let Some(__req) = __s.get("required").and_then(|v| v.as_array()) {
                            __merged_required.extend(__req.iter().cloned());
                        }
                    }
                }
            });
            quote! {
                let __base = serde_json::to_value(rmcp::schemars::schema_for!(#first))
                    .unwrap_or_else(|_| serde_json::json!({ "type": "object" }));
                let mut __merged_props = __base
                    .get("properties")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();
                let mut __merged_required = __base
                    .get("required")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                #(#merges)*
                serde_json::json!({
                    "type": "object",
                    "properties": serde_json::Value::Object(__merged_props),
                    "required": serde_json::Value::Array(__merged_required),
                })
            }
        }
    }
}

/// Generate the request-reconstruction prelude of the dispatch fn.
///
/// Emits statements that bind `__path: String` and `__body: Option<Vec<u8>>`
/// from the merged MCP `args` object, by deserializing each data type out of
/// `args` and re-projecting it:
/// * Path types → named-field substitution into `{param}` segments.
/// * Query types → `?k=v` (url-encoded, null-skipping) from the type's fields.
/// * Body type   → JSON-encoded request body.
fn build_rebuild(
    path_params: &[String],
    format_path: &str,
    path_types: &[Type],
    query_types: &[Type],
    body_types: &[Type],
) -> TokenStream {
    // ── path ──
    let path_construction = if path_params.is_empty() {
        quote! { let mut __path = #format_path.to_string(); }
    } else {
        // Collect named path-param values out of every Path<T> data type.
        let collectors = path_types.iter().map(|t| {
            quote! {
                if let Ok(__pv) = serde_json::from_value::<#t>(args.clone()) {
                    if let Ok(serde_json::Value::Object(__o)) = serde_json::to_value(&__pv) {
                        for (__k, __v) in __o {
                            __path_vals.insert(__k, __mcp_tool_macros_value_str(&__v));
                        }
                    }
                }
            }
        });
        let segment_args = path_params.iter().map(|p| {
            quote! {
                __path_vals.get(#p).cloned().unwrap_or_default()
            }
        });
        quote! {
            let mut __path_vals: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            #(#collectors)*
            let mut __path = format!(#format_path, #(#segment_args),*);
        }
    };

    // ── query ──
    let query_code = if query_types.is_empty() {
        quote! {}
    } else {
        let collectors = query_types.iter().map(|t| {
            quote! {
                if let Ok(__qv) = serde_json::from_value::<#t>(args.clone()) {
                    if let Ok(serde_json::Value::Object(__o)) = serde_json::to_value(&__qv) {
                        for (__k, __v) in __o {
                            if __v.is_null() {
                                continue;
                            }
                            __qp.push(format!(
                                "{}={}",
                                __mcp_tool_macros_encode(&__k),
                                __mcp_tool_macros_encode(&__mcp_tool_macros_value_str(&__v)),
                            ));
                        }
                    }
                }
            }
        });
        quote! {
            let mut __qp: Vec<String> = Vec::new();
            #(#collectors)*
            if !__qp.is_empty() {
                __path = format!("{}?{}", __path, __qp.join("&"));
            }
        }
    };

    // ── body ──
    let body_code = match body_types.first() {
        None => quote! { let __body: Option<Vec<u8>> = None; },
        Some(t) => quote! {
            let __body: Option<Vec<u8>> = serde_json::from_value::<#t>(args.clone())
                .ok()
                .and_then(|__b| serde_json::to_vec(&__b).ok());
        },
    };

    quote! {
        // Render a JSON scalar as its bare string form (strings unquoted).
        fn __mcp_tool_macros_value_str(v: &serde_json::Value) -> String {
            match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => String::new(),
                other => other.to_string(),
            }
        }
        // Minimal application/x-www-form-urlencoded-style percent encoding for
        // query keys/values (RFC 3986 unreserved kept; everything else %XX).
        fn __mcp_tool_macros_encode(s: &str) -> String {
            let mut out = String::with_capacity(s.len());
            for b in s.bytes() {
                match b {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        out.push(b as char);
                    }
                    _ => out.push_str(&format!("%{b:02X}")),
                }
            }
            out
        }
        #path_construction
        #query_code
        #body_code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn classifies_wrapper_types_and_excludes_extractors() {
        let p: syn::FnArg = parse_quote!(q: Query<SearchQuery>);
        assert!(matches!(classify(&p), ParamKind::Query(_)));
        let j: syn::FnArg = parse_quote!(body: Json<CreateReq>);
        assert!(matches!(classify(&j), ParamKind::Json(_)));
        let path: syn::FnArg = parse_quote!(p: Path<String>);
        assert!(matches!(classify(&path), ParamKind::Path(_)));
        let ex: syn::FnArg = parse_quote!(house: McpHouse);
        assert!(matches!(classify(&ex), ParamKind::Extractor));
        let ex2: syn::FnArg = parse_quote!(user: User);
        assert!(matches!(classify(&ex2), ParamKind::Extractor));
    }
}

#[cfg(test)]
mod expand_tests {
    use super::*;
    use quote::quote;

    #[test]
    fn expand_emits_schema_dispatch_and_submit() {
        let attr = quote! { name = "search_essence", description = "search" };
        let item = quote! {
            #[get("/api/mcp/search")]
            pub async fn search_essence_handler(
                house: McpHouse,
                Query(q): Query<SearchQuery>,
            ) -> Result<SearchEssenceResponse, EssenceError> { unreachable!() }
        };
        let out = expand(attr, item).to_string();
        assert!(out.contains("__search_essence_handler_input_schema"));
        assert!(out.contains("__search_essence_handler_dispatch"));
        assert!(out.contains("inventory :: submit"));
        assert!(out.contains("mcp_oneshot"));
        // original handler is preserved for the #[get] macro
        assert!(out.contains("search_essence_handler"));
    }

    #[test]
    fn expand_rejects_missing_route_attr() {
        let attr = quote! { name = "x", description = "y" };
        let item = quote! {
            pub async fn no_route_handler() -> Result<(), ()> { unreachable!() }
        };
        let out = expand(attr, item).to_string();
        assert!(out.contains("compile_error"));
    }
}
