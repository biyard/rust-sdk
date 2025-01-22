extern crate proc_macro;

mod api_model;
mod enum_prop;
mod query_display;
#[cfg(feature = "server")]
mod sql_model;

use api_model::api_model_impl;
use enum_prop::enum_prop_impl;
use proc_macro::TokenStream;
use query_display::query_display_impl;

#[proc_macro_derive(QueryDisplay)]
pub fn query_display_derive(input: TokenStream) -> TokenStream {
    query_display_impl(input)
}

#[proc_macro_attribute]
pub fn api_model(attr: TokenStream, item: TokenStream) -> TokenStream {
    api_model_impl(attr.into(), item.into()).into()
}

#[proc_macro_derive(EnumProp)]
pub fn enum_prop_derive(input: TokenStream) -> TokenStream {
    enum_prop_impl(input)
}
