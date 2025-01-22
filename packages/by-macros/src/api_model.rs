use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Field, Fields, Meta};

#[derive(Debug)]
enum ActionType {
    Summary,
    Queryable,
    Action(Vec<String>),
    ActionById(Vec<String>),
    Related(String),
    QueryActions(Vec<String>),
    ReadActions(Vec<String>),
}

/// Parse the attribute string and return the action type
/// The attribute should be in the form of #[api_model(action::action_name, action_by_id::action_name)]
///

fn parse_action_attr(attr: &Attribute) -> Vec<ActionType> {
    let mut types = Vec::new();

    if let Meta::List(meta_list) = attr.meta.clone() {
        if meta_list.path.is_ident("api_model") {
            let mut selected_action = ActionType::Summary;
            let mut selected_at = 0;
            let mut i = 0;

            for nested in meta_list.tokens.clone() {
                if let proc_macro2::TokenTree::Ident(iden) = nested {
                    let id = iden.to_string();
                    match id.as_str() {
                        "summary" => {
                            types.push(ActionType::Summary);
                        }
                        "queryable" => {
                            types.push(ActionType::Queryable);
                        }
                        "action" => {
                            selected_at = i;
                            selected_action = ActionType::Action(vec![]);
                        }
                        "action_by_id" => {
                            selected_at = i;
                            selected_action = ActionType::ActionById(vec![]);
                        }
                        "related" => {
                            selected_at = i;
                            selected_action = ActionType::Related("".to_string());
                        }
                        "query_action" => {
                            selected_at = i;
                            selected_action = ActionType::QueryActions(vec![]);
                        }
                        "read_action" => {
                            selected_at = i;
                            selected_action = ActionType::ReadActions(vec![]);
                        }
                        _ => {
                            if selected_at == (i - 2) {
                                match &selected_action {
                                    ActionType::Action(_) => {
                                        types.push(ActionType::Action(vec![id.to_string()]));
                                    }
                                    ActionType::ActionById(_) => {
                                        types.push(ActionType::ActionById(vec![id.to_string()]));
                                    }
                                    ActionType::Related(_) => {
                                        types.push(ActionType::Related(id.to_string()));
                                    }
                                    ActionType::QueryActions(_) => {
                                        types.push(ActionType::QueryActions(vec![id.to_string()]));
                                    }
                                    ActionType::ReadActions(_) => {
                                        types.push(ActionType::ReadActions(vec![id.to_string()]));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                } else if let proc_macro2::TokenTree::Group(group) = nested {
                    let mut actions = vec![];
                    for nested in group.stream() {
                        if let proc_macro2::TokenTree::Ident(iden) = nested {
                            let id = iden.to_string();
                            actions.push(id);
                        }
                    }

                    match &selected_action {
                        ActionType::Action(_) => {
                            types.push(ActionType::Action(actions));
                        }
                        ActionType::ActionById(_) => {
                            types.push(ActionType::ActionById(actions));
                        }
                        ActionType::QueryActions(_) => {
                            types.push(ActionType::QueryActions(actions));
                        }
                        ActionType::ReadActions(_) => {
                            types.push(ActionType::ReadActions(actions));
                        }
                        ActionType::Related(_) | ActionType::Summary | ActionType::Queryable => {}
                    }
                }

                i += 1;
            }
        }
    }

    types
}

#[derive(Debug)]
enum ActionField {
    Fields(Vec<Field>),
    Related(String),
}

pub fn api_model_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let _ = tracing_subscriber::fmt::try_init();
    #[cfg(feature = "server")]
    let db_structs: proc_macro2::TokenStream =
        crate::sql_model::sql_model_impl(attr.clone(), item.clone()).into();
    #[cfg(not(feature = "server"))]
    let db_structs: proc_macro2::TokenStream = quote! {};

    tracing::debug!("generated db code: {}", db_structs.to_string());

    let input_cloned = item.clone();
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;
    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => panic!("api_mode can only be applied to structs"),
    };

    let attr_args = attr.to_string();
    let mut iter_type = "CommonQueryResponse".to_string();
    let mut base_endpoint = String::new();

    let mut summary_fields = Vec::new();
    let mut queryable_fields = Vec::new();
    let mut action_names = HashMap::<String, ActionField>::new();
    let mut action_by_id_names = HashMap::<String, ActionField>::new();
    let mut query_action_names = HashMap::<String, ActionField>::new();
    let mut read_action_names = HashMap::<String, ActionField>::new();
    let mut parent_ids = Vec::new();

    for arg in attr_args.split(',') {
        let parts: Vec<&str> = arg.split('=').collect();
        if parts.len() == 2 {
            let key = parts[0].trim();
            let value = parts[1].trim().trim_matches('"');
            match key {
                "base" => {
                    base_endpoint = value
                        .split('/')
                        .map(|v| {
                            if v.starts_with(':') {
                                parent_ids.push(
                                    v.trim_start_matches(':').to_string().to_case(Case::Snake),
                                );
                                "{}"
                            } else {
                                v
                            }
                        })
                        .collect::<Vec<&str>>()
                        .join("/");
                }
                "iter_type" => iter_type = value.to_string(),
                "read_action" => {
                    let value = value
                        .trim_matches('[')
                        .trim_matches(']')
                        .split(",")
                        .collect::<Vec<&str>>();
                    for v in value {
                        tracing::debug!("Read action: {}", v);
                        let v = v.trim();
                        read_action_names.insert(v.to_string(), ActionField::Fields(vec![]));
                    }
                }
                _ => {}
            }
        }
    }

    if let Fields::Named(named_fields) = fields {
        for field in &named_fields.named {
            for attr in &field.attrs {
                let mut actions = vec![];
                let mut related = None::<String>;

                for t in parse_action_attr(attr) {
                    match t {
                        ActionType::Summary => {
                            summary_fields.push(field.clone());
                        }
                        ActionType::Queryable => {
                            queryable_fields.push(field.clone());
                        }
                        ActionType::Action(action_names) => {
                            actions.push(ActionType::Action(action_names));
                        }
                        ActionType::ActionById(action_names) => {
                            actions.push(ActionType::ActionById(action_names));
                        }
                        ActionType::Related(st) => {
                            related = Some(st);
                        }
                        ActionType::QueryActions(action_names) => {
                            actions.push(ActionType::QueryActions(action_names));
                        }
                        ActionType::ReadActions(action_names) => {
                            actions.push(ActionType::ReadActions(action_names));
                        }
                    }
                }

                for action in actions {
                    match (related.clone(), action) {
                        (Some(st), ActionType::Action(actions)) => {
                            for action_name in actions {
                                action_names
                                    .entry(action_name)
                                    .or_insert_with(|| ActionField::Related(st.clone()));
                            }
                        }
                        (Some(st), ActionType::ActionById(actions)) => {
                            for action_name in actions {
                                action_by_id_names
                                    .entry(action_name)
                                    .or_insert_with(|| ActionField::Related(st.clone()));
                            }
                        }
                        (None, ActionType::Action(actions)) => {
                            for action_name in actions {
                                match action_names
                                    .entry(action_name)
                                    .or_insert_with(|| ActionField::Fields(vec![]))
                                {
                                    ActionField::Fields(v) => {
                                        v.push(field.clone());
                                    }

                                    _ => {
                                        panic!("Action should have fields")
                                    }
                                };
                            }
                        }
                        (None, ActionType::ActionById(actions)) => {
                            for action_name in actions {
                                match action_by_id_names
                                    .entry(action_name)
                                    .or_insert_with(|| ActionField::Fields(vec![]))
                                {
                                    ActionField::Fields(v) => {
                                        v.push(field.clone());
                                    }
                                    _ => {
                                        panic!("ActionById should have fields")
                                    }
                                };
                            }
                        }
                        (_, ActionType::QueryActions(actions)) => {
                            for action_name in actions {
                                match query_action_names
                                    .entry(action_name)
                                    .or_insert_with(|| ActionField::Fields(vec![]))
                                {
                                    ActionField::Fields(v) => {
                                        v.push(field.clone());
                                    }
                                    _ => {
                                        panic!("ActionById should have fields")
                                    }
                                };
                            }
                        }
                        (_, ActionType::ReadActions(actions)) => {
                            for action_name in actions {
                                match read_action_names
                                    .entry(action_name)
                                    .or_insert_with(|| ActionField::Fields(vec![]))
                                {
                                    ActionField::Fields(v) => {
                                        v.push(field.clone());
                                    }
                                    _ => {
                                        panic!("ActionById should have fields")
                                    }
                                };
                            }
                        }

                        _ => {}
                    }
                }
            }
        }
    }

    let summary_struct = generate_summary_struct(&struct_name, &summary_fields);
    let query_struct = generate_query_struct(
        &struct_name,
        &base_endpoint,
        &parent_ids,
        &iter_type,
        &queryable_fields,
        &query_action_names,
    );
    let read_action_struct = generate_read_struct(
        &struct_name,
        &base_endpoint,
        &parent_ids,
        &read_action_names,
    );
    let action_struct =
        generate_action_struct(&struct_name, &base_endpoint, &parent_ids, &action_names);
    let action_by_id_struct = generate_action_by_id_struct(
        &struct_name,
        &base_endpoint,
        &parent_ids,
        &action_by_id_names,
    );

    let client_impl = generate_client_impl(struct_name, &base_endpoint, &parent_ids, &iter_type);
    let input = parse_macro_input!(input_cloned as syn::ItemStruct);
    let stripped_input = strip_struct_attributes(&input);

    let output = quote! {
        #db_structs
        #stripped_input
        #action_struct
        #action_by_id_struct
        #summary_struct
        #query_struct
        #client_impl

        #read_action_struct
    };

    tracing::debug!("Generated code: {}", output.to_string());

    output.into()
}

fn generate_read_struct(
    struct_name: &syn::Ident,
    base_endpoint: &str,
    parent_ids: &[String],

    read_actions: &HashMap<String, ActionField>,
) -> proc_macro2::TokenStream {
    let base_endpoint_lit = syn::LitStr::new(base_endpoint, struct_name.span());
    let read_action_struct_name =
        syn::Ident::new(&format!("{}ReadAction", struct_name), struct_name.span());
    let client_name = syn::Ident::new(&format!("{}Client", struct_name), struct_name.span());
    let parent_params = parent_ids.iter().map(|id| {
        let id = syn::Ident::new(id, struct_name.span());
        quote! { #id: &str, }
    });
    let parent_names = parent_ids.iter().map(|id| {
        let id = syn::Ident::new(id, struct_name.span());
        quote! { #id, }
    });

    let mut hashed_fields = HashSet::new();
    let mut query_builder_functions = vec![];
    let mut cli_read_action_functions = vec![];

    let mut query_fields = vec![];
    let mut read_action_types = vec![];

    for (read_action, fields) in read_actions {
        let mut params = vec![];
        let mut replace_expressions = vec![];

        match fields {
            ActionField::Fields(v) => {
                for field in v {
                    let field_name = &field.ident;
                    let field_type = &field.ty;

                    replace_expressions.push(quote! {
                        self.#field_name = Some(#field_name);
                    });
                    params.push((field_name.clone(), field_type.clone()));
                    if hashed_fields.contains(field_name) {
                        continue;
                    }
                    hashed_fields.insert(field_name.clone());
                    query_fields.push(quote! {
                        pub #field_name: Option<#field_type>,
                    });
                }
            }
            _ => {
                panic!("Related field should not be in queryable fields");
            }
        }

        let read_action_name =
            syn::Ident::new(&read_action.to_case(Case::Pascal), struct_name.span());
        read_action_types.push(quote! { #read_action_name, });

        let function_name = syn::Ident::new(&read_action.to_case(Case::Snake), struct_name.span());
        let function_params = params
            .iter()
            .map(|(field_name, field_type)| quote! { #field_name: #field_type, });

        let read_action_enum_name = syn::Ident::new(
            &format!("{}ReadActionType", struct_name),
            struct_name.span(),
        );

        query_builder_functions.push(quote! {
            pub fn #function_name(mut self, #(#function_params)*) -> Self {
                #(#replace_expressions)*
                self.action = Some(#read_action_enum_name::#read_action_name);
                self
            }
        });
        let function_params = params
            .iter()
            .map(|(field_name, field_type)| quote! { #field_name: #field_type, });
        let field_names = params.iter().map(|(field_name, _)| quote! { #field_name, });

        let parent_params = parent_params.clone();
        let parent_names = parent_names.clone();

        cli_read_action_functions.push(quote! {
            pub async fn #function_name(
                &self,
                #(#parent_params)*
                #(#function_params)*
            ) -> crate::Result<#struct_name> {
                let path = format!(#base_endpoint_lit, #(#parent_names)*);
                let endpoint = format!("{}{}", self.endpoint, path);
                let params = #read_action_struct_name::new()
                    .#function_name(#(#field_names)*);
                let query = format!("{}?{}", endpoint, params);
                rest_api::get(&query).await
            }
        })
    }

    let (read_action_enum, read_action_type_field) = if read_action_types.len() > 0 {
        let read_action_enum_name = syn::Ident::new(
            &format!("{}ReadActionType", struct_name),
            struct_name.span(),
        );
        (
            quote! {
                #[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
                #[serde(rename_all = "kebab-case")]
                #[cfg_attr(feature = "server", derive(schemars::JsonSchema, aide::OperationIo))]
                pub enum #read_action_enum_name {
                    #(#read_action_types)*
                }
            },
            quote! {
                pub action: Option<#read_action_enum_name>,
            },
        )
    } else {
        (quote! {}, quote! {})
    };

    quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Eq, PartialEq, by_macros::QueryDisplay)]
        #[cfg_attr(feature = "server", derive(schemars::JsonSchema, aide::OperationIo))]
        pub struct #read_action_struct_name {
            #read_action_type_field
            #(#query_fields)*
        }

        impl #read_action_struct_name {
            pub fn new() -> Self {
                Self::default()
            }

            #(#query_builder_functions)*
        }

        #read_action_enum


        impl #client_name {
            #(#cli_read_action_functions)*
        }
    }
}

fn generate_action_by_id_struct(
    struct_name: &syn::Ident,
    base_endpoint: &str,
    parent_ids: &[String],
    actions: &HashMap<String, ActionField>,
) -> proc_macro2::TokenStream {
    if actions.is_empty() {
        return quote! {};
    }
    let action_type = "ByIdAction";
    let action_name = syn::Ident::new(
        &format!("{}{}", struct_name, action_type),
        struct_name.span(),
    );
    let client_name = syn::Ident::new(&format!("{}Client", struct_name), struct_name.span());
    let base_endpoint_lit = syn::LitStr::new(base_endpoint, struct_name.span());
    let parent_params = parent_ids.iter().map(|id| {
        let id = syn::Ident::new(id, struct_name.span());
        quote! { #id: &str, }
    });
    let parent_names = parent_ids.iter().map(|id| {
        let id = syn::Ident::new(id, struct_name.span());
        quote! { #id, }
    });

    let mut action_fields = vec![];
    let mut action_requests = vec![];
    let mut cli_actions = vec![];

    for (k, v) in actions.iter() {
        let act = syn::Ident::new(&k.to_case(Case::Pascal), struct_name.span());
        let cli_act = syn::Ident::new(&k.to_case(Case::Snake), struct_name.span());
        let request_struct_name = match v {
            ActionField::Related(st) => syn::Ident::new(&st, struct_name.span()),
            _ => syn::Ident::new(
                &format!("{}{}Request", struct_name, act),
                struct_name.span(),
            ),
        };
        action_fields.push(quote! {
            #act(#request_struct_name),
        });

        if let ActionField::Fields(v) = v {
            let fields = v.iter().map(|field| {
                let field_name = &field.ident;
                let field_type = &field.ty;
                quote! { pub #field_name: #field_type, }
            });

            let params = v.iter().map(|field| {
                let field_name = &field.ident;
                let field_type = &field.ty;
                quote! { #field_name: #field_type, }
            });

            let field_names = v.iter().map(|field| {
                let field_name = &field.ident;
                quote! { #field_name, }
            });

            action_requests.push(quote! {
                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Eq, PartialEq)]
                #[cfg_attr(feature = "server", derive(schemars::JsonSchema, aide::OperationIo))]
                pub struct #request_struct_name {
                    #(#fields)*
                }
            });

            let parent_params = parent_params.clone();
            let parent_names = parent_names.clone();

            cli_actions.push(quote! {
                pub async fn #cli_act(&self, #(#parent_params)* id: &str, #(#params)*) -> crate::Result<#struct_name> {
                    let path = format!(#base_endpoint_lit, #(#parent_names)*);
                    let endpoint = format!("{}{}/{}", self.endpoint, path, id);
                    let req = #action_name::#act(#request_struct_name {
                        #(#field_names)*
                    });
                    rest_api::post(&endpoint, req).await
                }

            })
        } else if let ActionField::Related(st) = v {
            let parent_params = parent_params.clone();
            let parent_names = parent_names.clone();
            let req_type = syn::Ident::new(&st, struct_name.span());

            cli_actions.push(quote! {
                pub async fn #cli_act(&self, #(#parent_params)* id: &str, request: #req_type) -> crate::Result<#struct_name> {
                    let path = format!(#base_endpoint_lit, #(#parent_names)*);
                    let endpoint = format!("{}{}/{}", self.endpoint, path, id);

                    rest_api::post(&endpoint, request).await
                }

            })
        }
    }

    quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
        #[serde(rename_all = "snake_case")]
        #[cfg_attr(feature = "server", derive(schemars::JsonSchema, aide::OperationIo))]
        pub enum #action_name {
            #(#action_fields)*
        }

        #(#action_requests)*

        impl #client_name {
            pub async fn act_by_id(&self, #(#parent_params)* id: &str, params: #action_name) -> crate::Result<#struct_name> {
                let path = format!(#base_endpoint_lit, #(#parent_names)*);
                let endpoint = format!("{}{}/{}", self.endpoint, path, id);
                rest_api::post(&endpoint, params).await
            }

            #(#cli_actions)*
        }
    }
}

fn generate_action_struct(
    struct_name: &syn::Ident,
    base_endpoint: &str,
    parent_ids: &[String],
    actions: &HashMap<String, ActionField>,
) -> proc_macro2::TokenStream {
    if actions.is_empty() {
        return quote! {};
    }

    let action_type = "Action";
    let action_name = syn::Ident::new(
        &format!("{}{}", struct_name, action_type),
        struct_name.span(),
    );
    let client_name = syn::Ident::new(&format!("{}Client", struct_name), struct_name.span());
    let base_endpoint_lit = syn::LitStr::new(base_endpoint, struct_name.span());
    let parent_params = parent_ids.iter().map(|id| {
        let id = syn::Ident::new(id, struct_name.span());
        quote! { #id: &str, }
    });
    let parent_names = parent_ids.iter().map(|id| {
        let id = syn::Ident::new(id, struct_name.span());
        quote! { #id, }
    });

    let mut action_fields = vec![];
    let mut action_requests = vec![];
    let mut cli_actions = vec![];

    for (k, v) in actions.iter() {
        let act = syn::Ident::new(&k.to_case(Case::Pascal), struct_name.span());
        let cli_act = syn::Ident::new(&k.to_case(Case::Snake), struct_name.span());
        let request_struct_name = match v {
            ActionField::Related(st) => syn::Ident::new(&st, struct_name.span()),
            _ => syn::Ident::new(
                &format!("{}{}Request", struct_name, act),
                struct_name.span(),
            ),
        };
        action_fields.push(quote! {
            #act(#request_struct_name),
        });

        if let ActionField::Fields(v) = v {
            let fields = v.iter().map(|field| {
                let field_name = &field.ident;
                let field_type = &field.ty;
                quote! { pub #field_name: #field_type, }
            });

            let params = v.iter().map(|field| {
                let field_name = &field.ident;
                let field_type = &field.ty;
                quote! { #field_name: #field_type, }
            });

            let field_names = v.iter().map(|field| {
                let field_name = &field.ident;
                quote! { #field_name, }
            });

            action_requests.push(quote! {
                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Eq, PartialEq)]
                #[cfg_attr(feature = "server", derive(schemars::JsonSchema, aide::OperationIo))]
                pub struct #request_struct_name {
                    #(#fields)*
                }
            });
            let parent_params = parent_params.clone();
            let parent_names = parent_names.clone();

            cli_actions.push(quote! {
                pub async fn #cli_act(&self, #(#parent_params)* #(#params)*) -> crate::Result<#struct_name> {
                    let path = format!(#base_endpoint_lit, #(#parent_names)*);
                    let endpoint = format!("{}{}", self.endpoint, path);

                    let req = #action_name::#act(#request_struct_name {
                        #(#field_names)*
                    });
                    rest_api::post(&endpoint, req).await
                }

            })
        } else if let ActionField::Related(st) = v {
            let parent_params = parent_params.clone();
            let parent_names = parent_names.clone();
            let req_type = syn::Ident::new(&st, struct_name.span());

            cli_actions.push(quote! {
                pub async fn #cli_act(&self, #(#parent_params)* request: #req_type) -> crate::Result<#struct_name> {
                    let path = format!(#base_endpoint_lit, #(#parent_names)*);
                    let endpoint = format!("{}{}", self.endpoint, path);

                    rest_api::post(&endpoint, request).await
                }

            })
        }
    }

    quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
        #[serde(rename_all = "snake_case")]
        #[cfg_attr(feature = "server", derive(schemars::JsonSchema, aide::OperationIo))]
        pub enum #action_name {
            #(#action_fields)*
        }

        #(#action_requests)*

        impl #client_name {
            pub async fn act(&self, #(#parent_params)* params: #action_name) -> crate::Result<#struct_name> {
                let path = format!(#base_endpoint_lit, #(#parent_names)*);
                let endpoint = format!("{}{}", self.endpoint, path);
                rest_api::post(&endpoint, params).await
            }

            #(#cli_actions)*
        }
    }
}

fn generate_summary_struct(
    struct_name: &syn::Ident,
    summary_fields: &[syn::Field],
) -> proc_macro2::TokenStream {
    let summary_name = syn::Ident::new(&format!("{}Summary", struct_name), struct_name.span());
    let fields = summary_fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;
        quote! { pub #field_name: #field_type, }
    });

    quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Eq, PartialEq)]
        #[cfg_attr(feature = "server", derive(schemars::JsonSchema, aide::OperationIo))]
        pub struct #summary_name {
            #(#fields)*
        }
    }
}

fn generate_query_struct(
    struct_name: &syn::Ident,
    base_endpoint: &str,
    parent_ids: &[String],
    iter_type: &str,

    queryable_fields: &[syn::Field],
    read_actions: &HashMap<String, ActionField>,
) -> proc_macro2::TokenStream {
    let summary_name = syn::Ident::new(&format!("{}Summary", struct_name), struct_name.span());
    let client_name = syn::Ident::new(&format!("{}Client", struct_name), struct_name.span());
    let query_name = syn::Ident::new(&format!("{}Query", struct_name), struct_name.span());
    let base_endpoint_lit = syn::LitStr::new(base_endpoint, struct_name.span());
    let iter_type_with_summary = format!("{}<{}>", iter_type, summary_name);
    let iter_type_tokens: proc_macro2::TokenStream = iter_type_with_summary.parse().unwrap();

    let mut hashed_fields = HashSet::new();
    let mut query_fields = vec![];
    let mut query_builder_functions = vec![];
    let mut cli_read_action_functions = vec![];

    for field in queryable_fields {
        let field_name = &field.ident;
        hashed_fields.insert(field_name.clone());
        let field_type = &field.ty;
        query_fields.push(quote! { pub #field_name: Option<#field_type>, });
        let function_name = syn::Ident::new(
            &format!("with_{}", field_name.as_ref().unwrap()),
            struct_name.span(),
        );

        query_builder_functions.push(quote! {
            pub fn #function_name(mut self, #field_name: #field_type) -> Self {
                self.#field_name = Some(#field_name);
                self
            }
        });
    }
    let mut extended_query_fields = vec![];
    let mut read_action_types = vec![];

    for (read_action, fields) in read_actions {
        let mut params = vec![];
        let mut replace_expressions = vec![];

        match fields {
            ActionField::Fields(v) => {
                for field in v {
                    let field_name = &field.ident;
                    let field_type = &field.ty;

                    replace_expressions.push(quote! {
                        self.#field_name = Some(#field_name);
                    });
                    params.push((field_name.clone(), field_type.clone()));
                    if hashed_fields.contains(field_name) {
                        continue;
                    }
                    hashed_fields.insert(field_name.clone());
                    extended_query_fields.push(quote! {
                        pub #field_name: Option<#field_type>,
                    });
                }
            }
            _ => {
                panic!("Related field should not be in queryable fields");
            }
        }

        let read_action_name =
            syn::Ident::new(&read_action.to_case(Case::Pascal), struct_name.span());
        read_action_types.push(quote! { #read_action_name, });

        let function_name = syn::Ident::new(&read_action.to_case(Case::Snake), struct_name.span());
        let function_params = params.iter().map(|(field_name, field_type)| {
            quote! { #field_name: #field_type, }
        });
        let read_action_enum_name = syn::Ident::new(
            &format!("{}QueryActionType", struct_name),
            struct_name.span(),
        );

        query_builder_functions.push(quote! {
            pub fn #function_name(mut self, #(#function_params)*) -> Self {
                #(#replace_expressions)*
                self.action = Some(#read_action_enum_name::#read_action_name);
                self
            }
        });

        let function_params = params
            .iter()
            .map(|(field_name, field_type)| quote! { #field_name: #field_type, });
        let field_names = params
            .iter()
            .map(|(field_name, _)| quote! { #field_name: Some(#field_name), });
        let parent_params = parent_ids.iter().map(|id| {
            let id = syn::Ident::new(id, struct_name.span());
            quote! { #id: &str, }
        });
        let parent_names = parent_ids.iter().map(|id| {
            let id = syn::Ident::new(id, struct_name.span());
            quote! { #id, }
        });
        cli_read_action_functions.push(quote! {
            pub async fn #function_name(
                &self,
                size: usize,
                bookmark: Option<String>,
                #(#parent_params)*
                #(#function_params)*
            ) -> crate::Result<#iter_type_tokens> {
                let path = format!(#base_endpoint_lit, #(#parent_names)*);
                let endpoint = format!("{}{}", self.endpoint, path);
                let params = #query_name {
                    size,
                    bookmark,
                    action: Some(#read_action_enum_name::#read_action_name),
                    #(#field_names)*
                    ..#query_name::default()
                };
                let query = format!("{}?{}", endpoint, params);
                rest_api::get(&query).await
            }
        })
    }

    let (read_action_enum, read_action_type_field) = if read_action_types.len() > 0 {
        let read_action_enum_name = syn::Ident::new(
            &format!("{}QueryActionType", struct_name),
            struct_name.span(),
        );
        (
            quote! {
                #[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
                #[serde(rename_all = "kebab-case")]
                #[cfg_attr(feature = "server", derive(schemars::JsonSchema, aide::OperationIo))]
                pub enum #read_action_enum_name {
                    #(#read_action_types)*
                }
            },
            quote! {
                pub action: Option<#read_action_enum_name>,
            },
        )
    } else {
        (quote! {}, quote! {})
    };

    quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Eq, PartialEq, by_macros::QueryDisplay)]
        #[cfg_attr(feature = "server", derive(schemars::JsonSchema, aide::OperationIo))]
        pub struct #query_name {
            pub size: usize,
            pub bookmark: Option<String>,
            #read_action_type_field
            #(#query_fields)*
            #(#extended_query_fields)*
        }

        impl #query_name {
            pub fn new(size: usize) -> Self {
                Self {
                    size,
                    ..Self::default()
                }
            }

            pub fn with_bookmark(mut self, bookmark: String) -> Self {
                self.bookmark = Some(bookmark);
                self
            }

            #(#query_builder_functions)*
        }

        #read_action_enum

        impl #client_name {
            #(#cli_read_action_functions)*
        }
    }
}

fn generate_client_impl(
    struct_name: &syn::Ident,
    base_endpoint: &str,
    parent_ids: &[String],
    iter_type: &str,
) -> proc_macro2::TokenStream {
    let client_name = syn::Ident::new(&format!("{}Client", struct_name), struct_name.span());
    let query_name = syn::Ident::new(&format!("{}Query", struct_name), struct_name.span());
    let summary_name = syn::Ident::new(&format!("{}Summary", struct_name), struct_name.span());

    let base_endpoint_lit = syn::LitStr::new(base_endpoint, struct_name.span());

    let iter_type_with_summary = format!("{}<{}>", iter_type, summary_name);
    let iter_type_tokens: proc_macro2::TokenStream = iter_type_with_summary.parse().unwrap();
    let parent_params = parent_ids.iter().map(|id| {
        let id = syn::Ident::new(id, struct_name.span());
        quote! { #id: &str, }
    });
    let parent_names = parent_ids.iter().map(|id| {
        let id = syn::Ident::new(id, struct_name.span());
        quote! { #id, }
    });
    let parent_names_for_get = parent_names.clone();
    let parent_params_for_get = parent_params.clone();

    quote! {
        impl #struct_name {
            pub fn get_client(endpoint: &str) -> #client_name {
                #client_name { endpoint: endpoint.to_string() }
            }
        }

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Eq, PartialEq)]
        pub struct #client_name {
            pub endpoint: String,
        }

        impl #client_name {
            pub async fn query(
                &self,
                #(#parent_params)*
                params: #query_name,
            ) -> crate::Result<#iter_type_tokens> {
                let path = format!(#base_endpoint_lit, #(#parent_names)*);
                let endpoint = format!("{}{}", self.endpoint, path);
                let query = format!("{}?{}", endpoint, params);
                rest_api::get(&query).await
            }

            pub async fn get(&self, #(#parent_params_for_get)* id: &str) -> crate::Result<#struct_name> {
                let path = format!(#base_endpoint_lit, #(#parent_names_for_get)*);
                let endpoint = format!("{}{}/{}", self.endpoint, path, id);
                rest_api::get(&endpoint).await
            }
        }
    }
}

fn strip_struct_attributes(input: &syn::ItemStruct) -> syn::ItemStruct {
    let mut stripped_struct = input.clone();
    stripped_struct.fields = strip_api_model_attributes(&input.fields);
    stripped_struct
}

fn strip_api_model_attributes(fields: &syn::Fields) -> syn::Fields {
    match fields {
        syn::Fields::Named(named_fields) => {
            let stripped_fields = named_fields
                .named
                .iter()
                .map(|field| {
                    let mut field = field.clone();
                    field.attrs = field
                        .attrs
                        .iter()
                        .filter(|attr| !attr.path().is_ident("api_model"))
                        .cloned()
                        .collect();
                    field
                })
                .collect();
            syn::Fields::Named(syn::FieldsNamed {
                brace_token: named_fields.brace_token,
                named: stripped_fields,
            })
        }
        syn::Fields::Unnamed(_) | syn::Fields::Unit => fields.clone(),
    }
}
