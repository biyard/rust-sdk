//! Rust `syn::Type` → TypeScript type string.
//!
//! Mapping rules (PLAN.md §"타입 매핑 엣지케이스"):
//!   - `Option<T>`            → `T | null`
//!   - `Vec<T>` / `[T]`       → `T[]`
//!   - `String` / `&str`      → `string`
//!   - `i8..i128`,`u8..u128`,`f32`,`f64`,`isize`,`usize` → `number`
//!   - `bool`                 → `boolean`
//!   - `()`                   → `void`
//!   - `HashMap<K,V>` / `BTreeMap<K,V>` → `Record<K, V>`
//!   - `Form<T>`              → `T` (unwrapped before this is called, but
//!                              handled here too for safety)
//!   - named types            → last path-segment ident (ts-rs name)
//!   - `Result<T, _>`         → `T`
//!
//! The set of *named* types referenced is collected separately for import
//! generation (see `imports_of`).

use std::collections::BTreeSet;
use syn::{GenericArgument, PathArguments, Type};

/// Render a Rust type as a TypeScript type string.
pub fn ts_type(ty: &Type) -> String {
    match ty {
        Type::Path(tp) => {
            let seg = match tp.path.segments.last() {
                Some(s) => s,
                None => return "unknown".to_string(),
            };
            let ident = seg.ident.to_string();
            match ident.as_str() {
                "String" | "str" => "string".to_string(),
                "char" => "string".to_string(),
                "bool" => "boolean".to_string(),
                "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
                | "u128" | "usize" | "f32" | "f64" => "number".to_string(),
                "Option" => {
                    let inner = first_generic(seg).map(ts_type).unwrap_or_else(|| "unknown".into());
                    format!("{inner} | null")
                }
                "Vec" | "VecDeque" => {
                    let inner = first_generic(seg).map(ts_type).unwrap_or_else(|| "unknown".into());
                    format!("{}[]", maybe_paren(&inner))
                }
                "HashMap" | "BTreeMap" | "IndexMap" => {
                    let (k, v) = two_generics(seg);
                    let k = k.map(ts_type).unwrap_or_else(|| "string".into());
                    let v = v.map(ts_type).unwrap_or_else(|| "unknown".into());
                    format!("Record<{k}, {v}>")
                }
                "HashSet" | "BTreeSet" => {
                    let inner = first_generic(seg).map(ts_type).unwrap_or_else(|| "unknown".into());
                    format!("{}[]", maybe_paren(&inner))
                }
                "Form" => first_generic(seg).map(ts_type).unwrap_or_else(|| "unknown".into()),
                "Result" => first_generic(seg).map(ts_type).unwrap_or_else(|| "void".into()),
                "Box" | "Arc" | "Rc" => {
                    first_generic(seg).map(ts_type).unwrap_or_else(|| "unknown".into())
                }
                // A named domain type — ts-rs emits a type by this exact ident.
                _ => ident,
            }
        }
        Type::Reference(r) => ts_type(&r.elem),
        Type::Slice(s) => format!("{}[]", maybe_paren(&ts_type(&s.elem))),
        Type::Array(a) => format!("{}[]", maybe_paren(&ts_type(&a.elem))),
        Type::Tuple(t) if t.elems.is_empty() => "void".to_string(),
        Type::Tuple(t) => {
            let parts: Vec<String> = t.elems.iter().map(ts_type).collect();
            format!("[{}]", parts.join(", "))
        }
        Type::Paren(p) => ts_type(&p.elem),
        Type::Group(g) => ts_type(&g.elem),
        _ => "unknown".to_string(),
    }
}

/// Wrap a union (`A | B`) in parens so `(A | null)[]` reads correctly.
fn maybe_paren(s: &str) -> String {
    if s.contains('|') {
        format!("({s})")
    } else {
        s.to_string()
    }
}

fn first_generic(seg: &syn::PathSegment) -> Option<&Type> {
    if let PathArguments::AngleBracketed(args) = &seg.arguments {
        for a in &args.args {
            if let GenericArgument::Type(t) = a {
                return Some(t);
            }
        }
    }
    None
}

fn two_generics(seg: &syn::PathSegment) -> (Option<&Type>, Option<&Type>) {
    let mut it = if let PathArguments::AngleBracketed(args) = &seg.arguments {
        args.args
            .iter()
            .filter_map(|a| match a {
                GenericArgument::Type(t) => Some(t),
                _ => None,
            })
            .collect::<Vec<_>>()
            .into_iter()
    } else {
        Vec::new().into_iter()
    };
    (it.next(), it.next())
}

/// Built-in TS type names that need no import.
fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "string"
            | "number"
            | "boolean"
            | "void"
            | "unknown"
            | "null"
            | "Record"
            | "Array"
            | "char"
    ) || name.chars().all(|c| !c.is_alphabetic())
}

/// Collect the set of *named* (non-builtin) type idents referenced by `ty`.
/// These are the ts-rs-emitted types the generated file must import.
pub fn imports_of(ty: &Type, out: &mut BTreeSet<String>) {
    match ty {
        Type::Path(tp) => {
            if let Some(seg) = tp.path.segments.last() {
                let ident = seg.ident.to_string();
                match ident.as_str() {
                    // Containers / scalars: descend into generics, don't import the container.
                    "Option" | "Vec" | "VecDeque" | "HashSet" | "BTreeSet" | "Box" | "Arc"
                    | "Rc" | "Form" | "Result" | "HashMap" | "BTreeMap" | "IndexMap" => {
                        if let PathArguments::AngleBracketed(args) = &seg.arguments {
                            for a in &args.args {
                                if let GenericArgument::Type(t) = a {
                                    imports_of(t, out);
                                }
                            }
                        }
                    }
                    "String" | "str" | "char" | "bool" | "i8" | "i16" | "i32" | "i64" | "i128"
                    | "isize" | "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "f32" | "f64" => {}
                    _ => {
                        if !is_builtin(&ident) {
                            out.insert(ident);
                        }
                    }
                }
            }
        }
        Type::Reference(r) => imports_of(&r.elem, out),
        Type::Slice(s) => imports_of(&s.elem, out),
        Type::Array(a) => imports_of(&a.elem, out),
        Type::Tuple(t) => {
            for e in &t.elems {
                imports_of(e, out);
            }
        }
        Type::Paren(p) => imports_of(&p.elem, out),
        Type::Group(g) => imports_of(&g.elem, out),
        _ => {}
    }
}
