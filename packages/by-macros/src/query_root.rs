use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, ExprPath, Ident, Token};

pub fn query_root_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as QueryRootInput);

    let methods = input.fields.iter().map(|field| {
        let field_name = &field.name;
        let type_name = &field.ty;
        quote! {
            async fn #field_name(&self) -> &#type_name {
                &#type_name
            }
        }
    });

    let expanded = quote! {
        #[::async_graphql::Object]
        impl QueryRoot {
            #(#methods)*
        }
    };

    TokenStream::from(expanded)
}

// Structs to parse macro input
use syn::parse::{Parse, ParseStream, Result};

struct QueryField {
    name: Ident,
    _arrow: Token![=>],
    ty: ExprPath,
}

impl Parse for QueryField {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(QueryField {
            name: input.parse()?,
            _arrow: input.parse()?,
            ty: input.parse()?,
        })
    }
}

struct QueryRootInput {
    fields: Punctuated<QueryField, Token![,]>,
}

impl Parse for QueryRootInput {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(QueryRootInput {
            fields: Punctuated::parse_terminated(input)?,
        })
    }
}
