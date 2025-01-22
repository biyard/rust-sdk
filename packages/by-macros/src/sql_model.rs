use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields, Meta};

pub fn sql_model_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let repo_name = syn::Ident::new(&format!("{name}Repository"), name.span());
    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("api_mode can only be applied to structs"),
    };

    let attrs = parse_sql_model(attr);
    let table_name = if let Some(SqlModel::Table(ref table_name)) = attrs.get(&SqlModelKey::Table) {
        table_name.to_string()
    } else {
        format!("{}s", name.to_string().to_case(Case::Lower))
    };

    let rename = if let SqlModel::Rename(rename) = attrs
        .get(&SqlModelKey::Rename)
        .unwrap_or(&SqlModel::Rename(Case::Snake))
    {
        rename
    } else {
        todo!()
    };

    let create_table_function = create_table_tokens(&table_name, rename.clone(), &fields);
    let drop_table_query = syn::LitStr::new(
        &format!("DROP TABLE IF EXISTS {};", table_name),
        proc_macro2::Span::call_site(),
    );

    let output = quote! {
        impl #name {
            pub fn get_repository<'a>(pool: &'a sqlx::Pool<sqlx::Postgres>) -> #repo_name<'a> {
                #repo_name::new(pool)
            }
        }

        pub struct #repo_name<'a> {
            pool: &'a sqlx::Pool<sqlx::Postgres>,
        }

        impl<'a> #repo_name<'a> {
            pub fn new(pool: &'a sqlx::Pool<sqlx::Postgres>) -> Self {
                Self { pool }
            }

            #create_table_function
            pub async fn drop_table(&self) -> std::result::Result<(), sqlx::Error> {
                sqlx::query(#drop_table_query)
                    .execute(self.pool)
                    .await?;

                Ok(())
            }
        }
    };

    tracing::debug!("Generated code: {}", output.to_string());

    output.into()
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum SqlAttributeKey {
    PrimaryKey,
    SqlType,
    ManyToMany,
    ManyToOne,
    OneToMany,
}

#[derive(Debug)]
enum SqlAttribute {
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
}

#[derive(Debug)]
struct SqlAttributes {
    pub attrs: HashMap<SqlAttributeKey, SqlAttribute>,
}

impl SqlAttributes {
    fn is_primary_key(&self) -> bool {
        self.attrs.contains_key(&SqlAttributeKey::PrimaryKey)
    }

    fn to_primary_type(&self, var_type: &syn::Type) -> Option<String> {
        match var_type {
            syn::Type::Path(ref type_path) => {
                let type_ident = type_path.path.segments.last().unwrap().ident.to_string();
                let type_str = if let Some(SqlAttribute::SqlType(type_str)) =
                    self.attrs.get(&SqlAttributeKey::SqlType)
                {
                    type_str.to_string()
                } else {
                    match type_ident.as_str() {
                        "u64" | "i64" => "BIGINT".to_string(),
                        "String" => "TEXT".to_string(),
                        "bool" => "BOOLEAN".to_string(),
                        "i32" => "INTEGER".to_string(),
                        "f64" => "DOUBLE PRECISION".to_string(),

                        _ => return None,
                    }
                };

                let type_str = if self.attrs.contains_key(&SqlAttributeKey::PrimaryKey) {
                    format!("{} PRIMARY KEY", type_str)
                } else {
                    type_str
                };

                Some(type_str)
            }
            _ => {
                tracing::debug!("field type: {:?}", var_type);
                None
            }
        }
    }

    fn to_field_name(&self, var_name: &str, case: Case) -> String {
        format!("{}", var_name.to_case(case))
    }

    fn to_field_type(&self, var_type: &syn::Type) -> Option<String> {
        if self.attrs.contains_key(&SqlAttributeKey::ManyToMany)
            || self.attrs.contains_key(&SqlAttributeKey::ManyToOne)
        {
            return None;
        }

        match var_type {
            syn::Type::Path(ref type_path) => {
                let type_ident = type_path.path.segments.last().unwrap().ident.to_string();
                let type_str = if let Some(SqlAttribute::SqlType(type_str)) =
                    self.attrs.get(&SqlAttributeKey::SqlType)
                {
                    type_str.to_string()
                } else {
                    match type_ident.as_str() {
                        "u64" | "i64" => "BIGINT NOT NULL".to_string(),
                        "String" => "TEXT NOT NULL".to_string(),
                        "bool" => "BOOLEAN NOT NULL".to_string(),
                        "i32" => "INTEGER NOT NULL".to_string(),
                        "f64" => "DOUBLE PRECISION NOT NULL".to_string(),
                        "Option<u64>" | "Option<i64>" => "BIGINT".to_string(),
                        "Option<String>" => "TEXT".to_string(),
                        "Option<bool>" => "BOOLEAN".to_string(),
                        "Option<i32>" => "INTEGER".to_string(),
                        "Option<f64>" => "DOUBLE PRECISION".to_string(),

                        _ => return None,
                    }
                };

                let type_str = if self.attrs.contains_key(&SqlAttributeKey::PrimaryKey) {
                    format!("{} PRIMARY KEY", type_str)
                } else {
                    type_str
                };

                Some(type_str)
            }
            _ => {
                tracing::debug!("field type: {:?}", var_type);
                None
            }
        }
    }

    fn get_field_query(&self, var_name: &str, case: Case, var_type: &syn::Type) -> Option<String> {
        let name = self.to_field_name(var_name, case);

        if let Some(SqlAttribute::ManyToOne {
            table_name,
            foreign_key,
            foreign_key_type,
        }) = self.attrs.get(&SqlAttributeKey::ManyToOne)
        {
            Some(format!(
                "{} {} NOT NULL, FOREIGN KEY ({}) REFERENCES {} ({}) ON DELETE CASCADE",
                // Foreign field
                name,
                foreign_key_type,
                // Foreign key
                name,
                table_name,
                foreign_key.to_case(case),
            ))
        } else if let Some(SqlAttribute::OneToMany { .. }) =
            self.attrs.get(&SqlAttributeKey::OneToMany)
        {
            None
        } else {
            self.to_field_type(var_type)
                .map(|field_type| format!("{} {}", name, field_type))
        }
    }

    fn to_relation_field_type(&self, var_type: &syn::Type) -> Option<String> {
        match var_type {
            syn::Type::Path(ref type_path) => {
                let type_ident = type_path.path.segments.last().unwrap().ident.to_string();
                let type_str = if let Some(SqlAttribute::SqlType(type_str)) =
                    self.attrs.get(&SqlAttributeKey::SqlType)
                {
                    type_str.to_string()
                } else {
                    match type_ident.as_str() {
                        "u64" | "i64" => "BIGINT NOT NULL".to_string(),
                        "String" => "TEXT NOT NULL".to_string(),
                        "bool" => "BOOLEAN NOT NULL".to_string(),
                        "i32" => "INTEGER NOT NULL".to_string(),
                        "f64" => "DOUBLE PRECISION NOT NULL".to_string(),
                        "Option<u64>" | "Option<i64>" => "BIGINT".to_string(),
                        "Option<String>" => "TEXT".to_string(),
                        "Option<bool>" => "BOOLEAN".to_string(),
                        "Option<i32>" => "INTEGER".to_string(),
                        "Option<f64>" => "DOUBLE PRECISION".to_string(),

                        _ => return None,
                    }
                };

                Some(type_str)
            }
            _ => {
                tracing::debug!("field type: {:?}", var_type);
                None
            }
        }
    }

    fn get_additional_query(
        &self,
        this_table_name: &str,
        this_primary_key_name: &str,
        this_primary_key_type: &str,
        var_name: &str,
        var_type: &syn::Type,
        case: Case,
    ) -> String {
        tracing::debug!("additional query for {var_name}");
        let mut query = "".to_string();
        tracing::debug!("attrs {:?}", self.attrs);

        let this_primary_key_type = this_primary_key_type.replace("PRIMARY KEY", "");

        if let Some(SqlAttribute::ManyToMany {
            table_name,
            foreign_table_name,
            foreign_key: foreign_primary_key,
            foreign_key_type,
        }) = self.attrs.get(&SqlAttributeKey::ManyToMany)
        {
            tracing::debug!("additional query for many to many relation: {var_name}");
            let this_key = format!("{}_{}", this_table_name, this_primary_key_name).to_case(case);
            let foreign_pk =
                format!("{}_{}", foreign_table_name, foreign_primary_key).to_case(case);

            query.push_str(&format!(
                "CREATE TABLE IF NOT EXISTS {} ({} {} NOT NULL, {} {} NOT NULL, {} PRIMARY KEY ({}, {}), FOREIGN KEY ({}) REFERENCES {} ({}) ON DELETE CASCADE, FOREIGN KEY ({}) REFERENCES {} ({}) ON DELETE CASCADE); CREATE INDEX IF NOT EXISTS idx_{}_{} ON {}({}); CREATE INDEX IF NOT EXISTS idx_{}_{} ON {}({});",
                // Table name for many to many relation
                table_name,
                // key for this table
                this_key,
                this_primary_key_type,
                // key for other table
                foreign_pk,
                foreign_key_type,
                // Additionaly fields
                match self.to_relation_field_type(var_type) {
                    Some(field_type) =>
                        format!("{} {},", self.to_field_name(var_name, case), &field_type),
                    None => "".to_string(),
                },
                // Composited primary key
                this_key,
                foreign_pk,
                // Foreign key for this table key
                this_key,
                this_table_name,
                this_primary_key_name.to_case(case),
                // Foreign key for other table
                foreign_pk,
                foreign_table_name,
                foreign_primary_key.to_case(case),
                // Index for this table key
                table_name,
                this_key.to_case(Case::Snake),
                table_name,
                this_key,
                // Index for foreign table key
                table_name,
                foreign_pk.to_case(Case::Snake),
                table_name,
                foreign_pk
            ));
        }

        if let Some(SqlAttribute::ManyToOne {
            table_name: foreign_table,
            foreign_key,
            ..
        }) = self.attrs.get(&SqlAttributeKey::ManyToOne)
        {
            tracing::debug!("additional query for many to one relation: {var_name}");
            query.push_str(&format!(
                "CREATE INDEX idx_{}_{} ON {}({});",
                // index name
                foreign_table,
                foreign_key.to_case(Case::Snake),
                // indexing field
                foreign_table,
                foreign_key.to_case(case),
            ));
        }
        query
    }
}

fn parse_field_attr(field: &Field) -> SqlAttributes {
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
                            _ => match opened {
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
                                OpenedOffset::None => {}
                            },
                        }
                    } else if let proc_macro2::TokenTree::Group(_group) = nested {
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

fn create_table_tokens(table_name: &str, case: Case, fields: &Fields) -> proc_macro2::TokenStream {
    let mut create_query_fields = vec![];
    let mut additional_queries = vec![];

    let fields = if let Fields::Named(named_fields) = fields {
        named_fields.named.clone()
    } else {
        return quote! {};
    };
    let mut primary_key_name = "id".to_string().to_case(case);
    let mut primary_key_type = "TEXT".to_string();

    for field in fields {
        let field = field.clone();
        let attrs = parse_field_attr(&field);

        let field_name = field.ident.unwrap();
        let field_type = field.ty;

        if attrs.is_primary_key() {
            primary_key_name = field_name.to_string().to_case(case);
            primary_key_type = attrs
                .to_primary_type(&field_type)
                .unwrap_or_else(|| "TEXT".to_string());
        }

        match attrs.get_field_query(&field_name.to_string(), case, &field_type) {
            Some(query) => {
                create_query_fields.push(query);
            }
            None => {}
        }

        additional_queries.push(attrs.get_additional_query(
            table_name,
            &primary_key_name,
            &primary_key_type,
            &field_name.to_string(),
            &field_type,
            case,
        ));
    }

    let create_query_ouput = syn::LitStr::new(
        &format!(
            "CREATE TABLE IF NOT EXISTS {} ({}); {}",
            table_name,
            create_query_fields.join(","),
            additional_queries.join("\n")
        ),
        proc_macro2::Span::call_site(),
    );

    quote! {
        pub async fn create_table(&self) -> std::result::Result<(), sqlx::Error> {
            for query in #create_query_ouput.replace("\n", "").split(";") {
                sqlx::query(query)
                    .execute(self.pool)
                    .await?;
            }

            Ok(())
        }

    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum SqlModelKey {
    Table,
    Rename,
}

enum SqlModel {
    Table(String),
    Rename(Case),
}

fn parse_sql_model(attr: TokenStream) -> HashMap<SqlModelKey, SqlModel> {
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
