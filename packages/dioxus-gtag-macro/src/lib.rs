extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitStr};

/// Derives `dioxus_gtag::GtagEvent`, turning a struct into a typed GA4 event.
///
/// The event name defaults to the snake_case struct name and can be overridden
/// with `#[gtag(name = "…")]`. Each field becomes an event param via serde
/// serialization (so every field type must implement `serde::Serialize`);
/// `#[gtag(rename = "…")]` changes a param key and `#[gtag(skip)]` omits a
/// field.
///
/// ```rust,ignore
/// #[derive(GtagEvent)]
/// #[gtag(name = "purchase")]
/// pub struct Purchase {
///     pub value: f64,
///     pub currency: String,
///     #[gtag(rename = "item_id")]
///     pub sku: String,
///     #[gtag(skip)]
///     pub internal: bool,
/// }
///
/// gtag.send(&Purchase { … });
/// ```
#[proc_macro_derive(GtagEvent, attributes(gtag))]
pub fn derive_gtag_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    let mut event_name: Option<String> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("gtag") {
            let parsed = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let lit: LitStr = meta.value()?.parse()?;
                    event_name = Some(lit.value());
                    Ok(())
                } else {
                    Err(meta.error("unsupported gtag attribute; expected `name`"))
                }
            });
            if let Err(e) = parsed {
                return e.to_compile_error().into();
            }
        }
    }
    let event_name = event_name.unwrap_or_else(|| to_snake_case(&struct_name.to_string()));

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            Fields::Unit => {
                return quote! {
                    impl ::dioxus_gtag::GtagEvent for #struct_name {
                        fn event_name(&self) -> &str { #event_name }
                        fn params(&self) -> ::dioxus_gtag::__private::serde_json::Value {
                            ::dioxus_gtag::__private::serde_json::Value::Object(
                                ::dioxus_gtag::__private::serde_json::Map::new(),
                            )
                        }
                    }
                }
                .into()
            }
            _ => {
                return syn::Error::new_spanned(
                    struct_name,
                    "GtagEvent requires named fields or a unit struct",
                )
                .to_compile_error()
                .into()
            }
        },
        _ => {
            return syn::Error::new_spanned(struct_name, "GtagEvent can only be derived for structs")
                .to_compile_error()
                .into()
        }
    };

    let mut inserts = vec![];
    for field in fields {
        let ident = field.ident.as_ref().unwrap();
        let mut key = ident.to_string();
        let mut skip = false;
        for attr in &field.attrs {
            if attr.path().is_ident("gtag") {
                let parsed = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        let lit: LitStr = meta.value()?.parse()?;
                        key = lit.value();
                        Ok(())
                    } else if meta.path.is_ident("skip") {
                        skip = true;
                        Ok(())
                    } else {
                        Err(meta.error("unsupported gtag attribute; expected `rename` or `skip`"))
                    }
                });
                if let Err(e) = parsed {
                    return e.to_compile_error().into();
                }
            }
        }
        if skip {
            continue;
        }
        // Fail fast in debug builds so serialization bugs surface during
        // testing; in release, analytics must never take the app down, so
        // fall back to null.
        inserts.push(quote! {
            params.insert(
                #key.to_string(),
                match ::dioxus_gtag::__private::serde_json::to_value(&self.#ident) {
                    Ok(value) => value,
                    Err(err) => {
                        if cfg!(debug_assertions) {
                            panic!(
                                "GtagEvent: failed to serialize param `{}`: {}",
                                #key, err
                            );
                        }
                        ::dioxus_gtag::__private::serde_json::Value::Null
                    }
                },
            );
        });
    }

    quote! {
        impl ::dioxus_gtag::GtagEvent for #struct_name {
            fn event_name(&self) -> &str {
                #event_name
            }

            fn params(&self) -> ::dioxus_gtag::__private::serde_json::Value {
                let mut params = ::dioxus_gtag::__private::serde_json::Map::new();
                #(#inserts)*
                ::dioxus_gtag::__private::serde_json::Value::Object(params)
            }
        }
    }
    .into()
}

fn to_snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}
