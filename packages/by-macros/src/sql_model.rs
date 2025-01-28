use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::{parse_macro_input, DeriveInput, Field, Meta};

use crate::api_model_struct::ApiModel;

pub fn sql_model_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let repo_name = syn::Ident::new(&format!("{name}Repository"), name.span());

    let model = ApiModel::new(&input, attr.clone());
    if model.database.is_none() {
        return quote! {}.into();
    }

    let create_table_function = model.queries();
    let drop_table_function = model.drop_function();
    let insert = model.insert_function();
    let find_one = model.find_one_function();
    let find = model.find_function();
    let from_trait = model.from_pg_row_trait();

    let output = quote! {
        impl #name {
            pub fn get_repository(pool: sqlx::Pool<sqlx::Postgres>) -> #repo_name {
                #repo_name::new(pool)
            }
        }

        #[derive(Debug, Clone)]
        pub struct #repo_name {
            pool: sqlx::Pool<sqlx::Postgres>,
        }

        impl #repo_name {
            pub fn new(pool: sqlx::Pool<sqlx::Postgres>) -> Self {
                Self { pool }
            }

            #create_table_function
            #drop_table_function
            #insert
            #find_one
            #find
        }

        #from_trait
    };

    tracing::debug!("Generated code: {}", output.to_string());

    output.into()
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum SqlAttributeKey {
    PrimaryKey,
    SqlType,
    ManyToMany,
    ManyToOne,
    OneToMany,
    Unique,
    Auto,
    Version,
    Nullable,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum AutoOperation {
    Insert,
    Update,
}

#[derive(Debug)]
pub enum SqlAttribute {
    PrimaryKey,
    SqlType(String),
    ManyToMany {
        table_name: String,
        foreign_table_name: String,
        foreign_key: String,
        foreign_key_type: String,
    },
    ManyToOne {
        table_name: String,
        foreign_key: String,
        foreign_key_type: String,
    },
    OneToMany {
        #[allow(dead_code)]
        table_name: String,
        foreign_key: String,
    },
    Unique,
    Auto(Vec<AutoOperation>),
    Version(String),
    Nullable,
}

#[derive(Debug)]
enum OpenedOffset {
    None,
    Type,
    ManyToMany,
    ManyToOne,
    OneToMany,
    ForeignTableName,
    ForeignKey,
    ForeignKeyType,
    Auto,
    Version,
}

#[derive(Debug)]
pub struct SqlAttributes {
    pub attrs: HashMap<SqlAttributeKey, SqlAttribute>,
}

pub fn parse_field_attr(field: &Field) -> SqlAttributes {
    let mut field_attrs = HashMap::new();
    let name = field
        .ident
        .clone()
        .unwrap()
        .to_string()
        .to_case(Case::Snake);

    for attr in &field.attrs {
        if let Meta::List(meta_list) = attr.meta.clone() {
            if meta_list.path.is_ident("api_model") {
                let mut opened = OpenedOffset::None;
                let mut relation = None;

                for nested in meta_list.tokens.clone() {
                    if let proc_macro2::TokenTree::Ident(iden) = nested {
                        let id = iden.to_string();
                        match id.as_str() {
                            "primary_key" => {
                                field_attrs
                                    .insert(SqlAttributeKey::PrimaryKey, SqlAttribute::PrimaryKey);
                            }
                            "nullable" => {
                                field_attrs
                                    .insert(SqlAttributeKey::Nullable, SqlAttribute::Nullable);
                            }
                            "unique" => {
                                field_attrs.insert(SqlAttributeKey::Unique, SqlAttribute::Unique);
                            }
                            "type" => {
                                opened = OpenedOffset::Type;
                            }
                            "many_to_many" => {
                                opened = OpenedOffset::ManyToMany;
                            }
                            "many_to_one" => {
                                opened = OpenedOffset::ManyToOne;
                            }
                            "one_to_many" => {
                                opened = OpenedOffset::OneToMany;
                            }
                            "foreign_key" => {
                                opened = OpenedOffset::ForeignKey;
                            }
                            "foreign_table_name" => {
                                opened = OpenedOffset::ForeignTableName;
                            }
                            "foreign_key_type" => {
                                opened = OpenedOffset::ForeignKeyType;
                            }
                            "auto" => {
                                opened = OpenedOffset::Auto;
                            }
                            "version" => {
                                opened = OpenedOffset::Version;
                            }
                            _ => match opened {
                                OpenedOffset::Version => {
                                    field_attrs.insert(
                                        SqlAttributeKey::Version,
                                        SqlAttribute::Version(id),
                                    );
                                }
                                OpenedOffset::Type => {
                                    field_attrs.insert(
                                        SqlAttributeKey::SqlType,
                                        SqlAttribute::SqlType(id),
                                    );
                                }
                                OpenedOffset::ManyToMany => {
                                    field_attrs.insert(
                                        SqlAttributeKey::ManyToMany,
                                        SqlAttribute::ManyToMany {
                                            table_name: id,
                                            foreign_table_name: "".to_string(),
                                            foreign_key: "id".to_string(),
                                            foreign_key_type: "TEXT".to_string(),
                                        },
                                    );
                                    relation = Some(SqlAttributeKey::ManyToMany);
                                    tracing::debug!("many_to_many: {name}");
                                }
                                OpenedOffset::ManyToOne => {
                                    field_attrs.insert(
                                        SqlAttributeKey::ManyToOne,
                                        SqlAttribute::ManyToOne {
                                            table_name: id,
                                            foreign_key: "id".to_string(),
                                            foreign_key_type: "TEXT".to_string(),
                                        },
                                    );
                                    relation = Some(SqlAttributeKey::ManyToOne);
                                    tracing::debug!("many_to_one: {name}");
                                }
                                OpenedOffset::OneToMany => {
                                    field_attrs.insert(
                                        SqlAttributeKey::OneToMany,
                                        SqlAttribute::OneToMany {
                                            table_name: id,
                                            foreign_key: "id".to_string(),
                                        },
                                    );
                                    relation = Some(SqlAttributeKey::OneToMany);
                                    tracing::debug!("one_to_many: {name}");
                                }
                                OpenedOffset::ForeignKey => match relation {
                                    Some(SqlAttributeKey::ManyToOne) => {
                                        field_attrs.get_mut(&SqlAttributeKey::ManyToOne).map(
                                            |attr| {
                                                if let SqlAttribute::ManyToOne {
                                                    ref mut foreign_key,
                                                    ..
                                                } = attr
                                                {
                                                    *foreign_key = id
                                                }
                                            },
                                        );
                                    }
                                    Some(SqlAttributeKey::ManyToMany) => {
                                        field_attrs.get_mut(&SqlAttributeKey::ManyToMany).map(
                                            |attr| {
                                                if let SqlAttribute::ManyToMany {
                                                    foreign_key: ref mut foreign_primary_key,
                                                    ..
                                                } = attr
                                                {
                                                    *foreign_primary_key = id
                                                }
                                            },
                                        );
                                    }
                                    Some(SqlAttributeKey::OneToMany) => {
                                        field_attrs.get_mut(&SqlAttributeKey::OneToMany).map(
                                            |attr| {
                                                if let SqlAttribute::OneToMany {
                                                    ref mut foreign_key,
                                                    ..
                                                } = attr
                                                {
                                                    *foreign_key = id
                                                }
                                            },
                                        );
                                    }
                                    _ => {
                                        tracing::error!("foreign_key must be defined after many_to_many, one_to_many: {name}");
                                    }
                                },
                                OpenedOffset::ForeignTableName => match relation {
                                    Some(SqlAttributeKey::ManyToMany) => {
                                        field_attrs.get_mut(&SqlAttributeKey::ManyToMany).map(
                                            |attr| {
                                                if let SqlAttribute::ManyToMany {
                                                    ref mut foreign_table_name,
                                                    ..
                                                } = attr
                                                {
                                                    *foreign_table_name = id
                                                }
                                            },
                                        );
                                    }
                                    _ => {
                                        tracing::error!("foreign_table_name must be defined after many_to_many: {name}");
                                    }
                                },
                                OpenedOffset::ForeignKeyType => match relation {
                                    Some(SqlAttributeKey::ManyToOne) => {
                                        field_attrs.get_mut(&SqlAttributeKey::ManyToOne).map(
                                            |attr| {
                                                if let SqlAttribute::ManyToOne {
                                                    ref mut foreign_key_type,
                                                    ..
                                                } = attr
                                                {
                                                    *foreign_key_type = id
                                                }
                                            },
                                        );
                                    }
                                    Some(SqlAttributeKey::ManyToMany) => {
                                        field_attrs.get_mut(&SqlAttributeKey::ManyToMany).map(
                                            |attr| {
                                                if let SqlAttribute::ManyToMany {
                                                    ref mut foreign_key_type,
                                                    ..
                                                } = attr
                                                {
                                                    *foreign_key_type = id
                                                }
                                            },
                                        );
                                    }
                                    _ => {
                                        tracing::error!("foreign_key_type must be defined after many_to_many, many_to_one: {name}");
                                    }
                                },
                                OpenedOffset::Auto => {
                                    let auto = match id.as_str() {
                                        "insert" => AutoOperation::Insert,
                                        "update" => AutoOperation::Update,
                                        _ => {
                                            tracing::error!("invalid auto operation: {id}");
                                            continue;
                                        }
                                    };

                                    field_attrs
                                        .entry(SqlAttributeKey::Auto)
                                        .or_insert_with(|| SqlAttribute::Auto(vec![]));

                                    if let Some(SqlAttribute::Auto(ref mut operations)) =
                                        field_attrs.get_mut(&SqlAttributeKey::Auto)
                                    {
                                        operations.push(auto);
                                    }
                                }
                                OpenedOffset::None => {}
                            },
                        }
                    } else if let proc_macro2::TokenTree::Group(group) = nested {
                        match opened {
                            OpenedOffset::Auto => {
                                for nested in group.stream() {
                                    if let proc_macro2::TokenTree::Ident(iden) = nested {
                                        let id = iden.to_string();

                                        field_attrs
                                            .entry(SqlAttributeKey::Auto)
                                            .or_insert_with(|| SqlAttribute::Auto(vec![]));

                                        if let Some(SqlAttribute::Auto(ref mut operations)) =
                                            field_attrs.get_mut(&SqlAttributeKey::Auto)
                                        {
                                            operations.push(match id.as_str() {
                                                "insert" => AutoOperation::Insert,
                                                "update" => AutoOperation::Update,
                                                _ => {
                                                    tracing::error!("invalid auto operation: {id}");
                                                    continue;
                                                }
                                            });
                                        }
                                    }
                                }

                                opened = OpenedOffset::None;
                            }
                            _ => {}
                        }
                    } else if let proc_macro2::TokenTree::Punct(punct) = nested {
                        if punct.to_string().as_str() == "," {
                            opened = OpenedOffset::None;
                        }
                    }
                }
            }
        }
    }

    SqlAttributes { attrs: field_attrs }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum SqlModelKey {
    Table,
    Rename,
}

pub enum SqlModel {
    Table(String),
    Rename(Case),
}

pub fn parse_sql_model(attr: TokenStream) -> HashMap<SqlModelKey, SqlModel> {
    let attr_args = attr.to_string();
    let mut models = HashMap::new();

    for arg in attr_args.split(',') {
        let parts: Vec<&str> = arg.split('=').collect();

        if parts.len() == 2 {
            let key = parts[0].trim();
            let value = parts[1].trim().trim_matches('"');

            match key {
                "table" => {
                    models.insert(SqlModelKey::Table, SqlModel::Table(value.to_string()));
                }
                "rename" => {
                    models.insert(
                        SqlModelKey::Rename,
                        match value {
                            "upcase" => SqlModel::Rename(Case::UpperSnake),
                            "camel" => SqlModel::Rename(Case::Camel),
                            "pascal" | "uppercamel" => SqlModel::Rename(Case::Pascal),
                            "snake" | "underscore" => SqlModel::Rename(Case::Snake),
                            "kebab" => SqlModel::Rename(Case::Kebab),
                            _ => {
                                panic!("invalid rename value {}", value);
                            }
                        },
                    );
                }
                _ => {}
            }
        }
    }

    models
}
