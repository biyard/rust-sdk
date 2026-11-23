use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    FnArg, GenericArgument, ItemFn, LitStr, Pat, PathArguments, ReturnType, Token, Type,
};

/// Parsed `#[grpc(service = "pkg.Service", version = "v1", method = "Name")]`.
struct GrpcArgs {
    service: LitStr,
    version: LitStr,
    method: Option<LitStr>,
}

impl Parse for GrpcArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut service = None;
        let mut version = None;
        let mut method = None;
        let pairs = Punctuated::<syn::MetaNameValue, Token![,]>::parse_terminated(input)?;
        for pair in pairs {
            let key = pair
                .path
                .get_ident()
                .ok_or_else(|| syn::Error::new_spanned(&pair.path, "expected identifier"))?
                .to_string();
            let value = match &pair.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) => s.clone(),
                other => return Err(syn::Error::new_spanned(other, "expected string literal")),
            };
            match key.as_str() {
                "service" => service = Some(value),
                "version" => version = Some(value),
                "method" => method = Some(value),
                other => {
                    return Err(syn::Error::new_spanned(
                        &pair.path,
                        format!("unknown attribute: {other}"),
                    ))
                }
            }
        }
        Ok(GrpcArgs {
            service: service.ok_or_else(|| input.error("missing `service` attribute"))?,
            version: version.ok_or_else(|| input.error("missing `version` attribute"))?,
            method,
        })
    }
}

/// Accepts buf's PACKAGE_VERSION_SUFFIX forms: `v1`, `v2`, `v1alpha1`, `v1beta2`.
fn is_valid_version(version: &str) -> bool {
    let Some(rest) = version.strip_prefix('v') else {
        return false;
    };
    let major_len = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if major_len == 0 || rest.starts_with('0') {
        return false;
    }
    let suffix = &rest[major_len..];
    if suffix.is_empty() {
        return true;
    }
    let Some(stage) = suffix
        .strip_prefix("alpha")
        .or_else(|| suffix.strip_prefix("beta"))
    else {
        return false;
    };
    !stage.is_empty() && !stage.starts_with('0') && stage.chars().all(|c| c.is_ascii_digit())
}

fn result_ok_type(output: &ReturnType) -> Option<&Type> {
    let ReturnType::Type(_, ty) = output else {
        return None;
    };
    let Type::Path(path) = ty.as_ref() else {
        return None;
    };
    let last = path.path.segments.last()?;
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };
    args.args.iter().find_map(|arg| match arg {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })
}

pub fn grpc_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = match syn::parse2::<GrpcArgs>(attr) {
        Ok(args) => args,
        Err(e) => return e.to_compile_error(),
    };
    let function = match syn::parse2::<ItemFn>(item) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error(),
    };

    if function.sig.asyncness.is_none() {
        return syn::Error::new_spanned(&function.sig.fn_token, "#[grpc] requires an async fn")
            .to_compile_error();
    }

    let version = args.version.value();
    if !is_valid_version(&version) {
        return syn::Error::new_spanned(
            &args.version,
            "version must look like v1, v2, v1alpha1 or v1beta1",
        )
        .to_compile_error();
    }

    let service = args.service.value();
    let Some((package, service_name)) = service.rsplit_once('.') else {
        return syn::Error::new_spanned(
            &args.service,
            "service must be `<package>.<Service>` without the version, e.g. `biyard.greeter.Greeter`",
        )
        .to_compile_error();
    };
    if package.split('.').any(is_valid_version) {
        return syn::Error::new_spanned(
            &args.service,
            "put the version in `version = \"..\"`, not in `service`",
        )
        .to_compile_error();
    }
    let full_service = format!("{package}.{version}.{service_name}");

    let fn_name = &function.sig.ident;
    let method = args
        .method
        .map(|m| m.value())
        .unwrap_or_else(|| fn_name.to_string().to_case(Case::Pascal));
    let path = format!("/{full_service}/{method}");

    let typed: Vec<&syn::PatType> = function
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Typed(t) => Some(t),
            FnArg::Receiver(_) => None,
        })
        .collect();
    if typed.len() != function.sig.inputs.len() {
        return syn::Error::new_spanned(&function.sig.inputs, "#[grpc] functions take no `self`")
            .to_compile_error();
    }
    let Some((request, extractors)) = typed.split_last() else {
        return syn::Error::new_spanned(
            &function.sig,
            "#[grpc] needs the request message as the last parameter",
        )
        .to_compile_error();
    };
    let request_ty = &request.ty;

    let Some(ok_ty) = result_ok_type(&function.sig.output) else {
        return syn::Error::new_spanned(
            &function.sig.output,
            "#[grpc] functions must return Result<Message | by_grpc::Reply<Message>, E: Into<tonic::Status>>",
        )
        .to_compile_error();
    };

    let mut arg_idents = Vec::new();
    let mut extract_stmts = Vec::new();
    for (i, extractor) in extractors.iter().enumerate() {
        let ident = match extractor.pat.as_ref() {
            Pat::Ident(p) => format_ident!("__grpc_{}", p.ident),
            _ => format_ident!("__grpc_arg{}", i),
        };
        let ty = &extractor.ty;
        extract_stmts.push(quote! {
            let #ident = <#ty as ::by_grpc::FromCall>::from_call(&__parts).await?;
        });
        arg_idents.push(ident);
    }

    let handler_name = format_ident!("__by_grpc_handler_{}", fn_name);

    quote! {
        #function

        #[doc(hidden)]
        #[allow(non_snake_case)]
        fn #handler_name(
            request: ::by_grpc::http::Request<::by_grpc::axum::body::Body>,
        ) -> ::by_grpc::HandlerFuture {
            ::by_grpc::unary::<#request_ty, <#ok_ty as ::by_grpc::IntoReply>::Message, _, _>(
                request,
                |__parts: ::by_grpc::CallParts, __message: #request_ty| async move {
                    #(#extract_stmts)*
                    let __out = #fn_name(#(#arg_idents,)* __message)
                        .await
                        .map_err(::core::convert::Into::<::by_grpc::tonic::Status>::into)?;
                    ::core::result::Result::Ok(::by_grpc::IntoReply::into_reply(__out))
                },
            )
        }

        ::by_grpc::inventory::submit! {
            ::by_grpc::GrpcMethod {
                service: #full_service,
                method: #method,
                version: #version,
                path: #path,
                request_type: <#request_ty as ::by_grpc::prost::Name>::full_name,
                response_type: <<#ok_ty as ::by_grpc::IntoReply>::Message as ::by_grpc::prost::Name>::full_name,
                handler: #handler_name,
                source: ::core::concat!(::core::module_path!(), "::", ::core::stringify!(#fn_name)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_valid_version;

    #[test]
    fn accepts_buf_version_suffixes() {
        for ok in ["v1", "v2", "v10", "v1alpha1", "v2beta3"] {
            assert!(is_valid_version(ok), "{ok}");
        }
        for bad in ["1", "v0", "v01", "v1alpha", "v1rc1", "V1", "v"] {
            assert!(!is_valid_version(bad), "{bad}");
        }
    }
}
