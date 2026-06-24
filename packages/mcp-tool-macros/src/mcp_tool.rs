use syn::{FnArg, GenericArgument, PathArguments, Type};

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
