use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    FnArg, GenericArgument, ItemFn, LitStr, Pat, PathArguments, ReturnType, Token, Type,
    TypeParamBound,
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
                        format!("unknown attribute `{other}`, expected service, version or method"),
                    ))
                }
            }
        }
        Ok(GrpcArgs {
            service: service.ok_or_else(|| {
                syn::Error::new(
                    Span::call_site(),
                    "missing `service`, e.g. service = \"biyard.greeter.Greeter\"",
                )
            })?,
            version: version.ok_or_else(|| {
                syn::Error::new(
                    Span::call_site(),
                    "missing `version`: every #[grpc] method belongs to a versioned proto package, e.g. version = \"v1\"",
                )
            })?,
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

/// Request side of a signature: one message or a stream of messages.
enum Input<'a> {
    Message(&'a Type),
    Stream(&'a Type),
}

/// Response side of a signature: the Ok type, and for streams the message type it yields.
enum Output<'a> {
    Message(&'a Type),
    Stream(&'a Type),
}

fn last_segment(ty: &Type) -> Option<&syn::PathSegment> {
    match ty {
        Type::Path(p) if p.qself.is_none() => p.path.segments.last(),
        _ => None,
    }
}

fn first_type_arg(segment: &syn::PathSegment) -> Option<&Type> {
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    args.args.iter().find_map(|arg| match arg {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })
}

fn generic_of<'a>(ty: &'a Type, name: &str) -> Option<&'a Type> {
    let segment = last_segment(ty)?;
    if segment.ident != name {
        return None;
    }
    first_type_arg(segment)
}

/// `M` from `impl Stream<Item = Result<M, _>> + ..`; `None` if `ty` is not `impl Stream`.
fn impl_stream_item(ty: &Type) -> Option<syn::Result<&Type>> {
    let Type::ImplTrait(imp) = ty else {
        return None;
    };
    let stream = imp.bounds.iter().find_map(|bound| match bound {
        TypeParamBound::Trait(t) => t.path.segments.last().filter(|s| s.ident == "Stream"),
        _ => None,
    })?;
    let item = match &stream.arguments {
        PathArguments::AngleBracketed(args) => args.args.iter().find_map(|arg| match arg {
            GenericArgument::AssocType(assoc) if assoc.ident == "Item" => Some(&assoc.ty),
            _ => None,
        }),
        _ => None,
    };
    Some(
        item.and_then(|item| generic_of(item, "Result"))
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    ty,
                    "stream items must be spelled `Result<Message, tonic::Status>`",
                )
            }),
    )
}

fn classify_input(ty: &Type) -> syn::Result<Input<'_>> {
    if let Some(req) = generic_of(ty, "Streaming") {
        return Ok(Input::Stream(req));
    }
    match impl_stream_item(ty) {
        Some(req) => Ok(Input::Stream(req?)),
        None if matches!(ty, Type::ImplTrait(_)) => Err(syn::Error::new_spanned(
            ty,
            "the request must be a prost message, `Streaming<Req>` or `impl Stream<Item = Result<Req, Status>>`",
        )),
        None => Ok(Input::Message(ty)),
    }
}

fn classify_output(ok: &Type) -> syn::Result<Output<'_>> {
    let inner = generic_of(ok, "Reply").unwrap_or(ok);
    if let Some(item) = generic_of(inner, "ResponseStream") {
        return Ok(Output::Stream(item));
    }
    match impl_stream_item(inner) {
        Some(item) => Ok(Output::Stream(item?)),
        None if matches!(inner, Type::ImplTrait(_)) => Err(syn::Error::new_spanned(
            inner,
            "a streamed response must be `impl Stream<Item = Result<M, Status>> + Send + 'static`",
        )),
        None => Ok(Output::Message(ok)),
    }
}

fn result_ok_type(output: &ReturnType) -> Option<&Type> {
    let ReturnType::Type(_, ty) = output else {
        return None;
    };
    first_type_arg(last_segment(ty)?)
}

pub fn grpc_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = match syn::parse2::<ItemFn>(item) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error(),
    };
    let expanded = syn::parse2::<GrpcArgs>(attr).and_then(|args| expand(args, &function));
    match expanded {
        Ok(tokens) => tokens,
        Err(e) => {
            let err = e.to_compile_error();
            quote! { #function #err }
        }
    }
}

fn expand(args: GrpcArgs, function: &ItemFn) -> syn::Result<TokenStream> {
    let sig = &function.sig;
    if sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            sig.fn_token,
            "#[grpc] requires an async fn",
        ));
    }
    if !sig.generics.params.is_empty() || sig.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &sig.generics,
            "#[grpc] functions cannot declare generics",
        ));
    }

    let version = args.version.value();
    if !is_valid_version(&version) {
        return Err(syn::Error::new_spanned(
            &args.version,
            "version must look like v1, v2, v1alpha1 or v1beta1",
        ));
    }

    let service = args.service.value();
    let Some((package, service_name)) = service.rsplit_once('.') else {
        return Err(syn::Error::new_spanned(
            &args.service,
            "service must be `<package>.<Service>` without the version, e.g. `biyard.greeter.Greeter`",
        ));
    };
    if package.split('.').any(is_valid_version) {
        return Err(syn::Error::new_spanned(
            &args.service,
            "put the version in `version = \"..\"`, not in `service`",
        ));
    }
    let full_service = format!("{package}.{version}.{service_name}");

    let fn_name = &sig.ident;
    let method = args
        .method
        .map(|m| m.value())
        .unwrap_or_else(|| fn_name.to_string().to_case(Case::Pascal));
    let path = format!("/{full_service}/{method}");

    let mut typed = Vec::new();
    for arg in &sig.inputs {
        match arg {
            FnArg::Typed(t) => typed.push(t),
            FnArg::Receiver(r) => {
                return Err(syn::Error::new_spanned(
                    r,
                    "#[grpc] functions take no `self`",
                ))
            }
        }
    }
    let Some((request, extractors)) = typed.split_last() else {
        return Err(syn::Error::new(
            sig.paren_token.span.join(),
            "#[grpc] needs the request message (or request stream) as the last parameter",
        ));
    };

    let Some(ok_ty) = result_ok_type(&sig.output) else {
        return Err(syn::Error::new(
            match &sig.output {
                ReturnType::Type(_, ty) => syn::spanned::Spanned::span(ty),
                ReturnType::Default => sig.paren_token.span.join(),
            },
            "#[grpc] functions return Result<M | Reply<M> | impl Stream<Item = Result<M, Status>>, E> with E: Into<tonic::Status>",
        ));
    };

    let input = classify_input(&request.ty)?;
    let output = classify_output(ok_ty)?;

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

    let (req_ty, message_param_ty) = match input {
        Input::Message(ty) => (ty, quote! { #ty }),
        Input::Stream(ty) => (ty, quote! { ::by_grpc::Streaming<#ty> }),
    };
    let (resp_ty, into_response) = match output {
        Output::Message(ok) => (
            quote! { <#ok as ::by_grpc::IntoReply>::Message },
            quote! { ::by_grpc::IntoReply::into_reply(__out) },
        ),
        Output::Stream(item) => (
            quote! { #item },
            quote! { ::by_grpc::IntoStreamReply::<#item>::into_stream_reply(__out) },
        ),
    };
    let (runner, kind) = match (&input, &output) {
        (Input::Message(_), Output::Message(_)) => (quote! { unary }, quote! { Unary }),
        (Input::Message(_), Output::Stream(_)) => {
            (quote! { server_streaming }, quote! { ServerStreaming })
        }
        (Input::Stream(_), Output::Message(_)) => {
            (quote! { client_streaming }, quote! { ClientStreaming })
        }
        (Input::Stream(_), Output::Stream(_)) => (quote! { bidi }, quote! { Bidi }),
    };

    let handler_name = format_ident!("__by_grpc_handler_{}", fn_name);

    Ok(quote! {
        #function

        #[doc(hidden)]
        #[allow(non_snake_case)]
        fn #handler_name(
            request: ::by_grpc::http::Request<::by_grpc::axum::body::Body>,
        ) -> ::by_grpc::HandlerFuture {
            ::by_grpc::#runner::<#req_ty, #resp_ty, _, _>(
                request,
                |__parts: ::by_grpc::CallParts, __message: #message_param_ty| async move {
                    #(#extract_stmts)*
                    let __out = #fn_name(#(#arg_idents,)* __message)
                        .await
                        .map_err(::core::convert::Into::<::by_grpc::tonic::Status>::into)?;
                    ::core::result::Result::Ok(#into_response)
                },
            )
        }

        ::by_grpc::inventory::submit! {
            ::by_grpc::GrpcMethod {
                service: #full_service,
                method: #method,
                version: #version,
                path: #path,
                kind: ::by_grpc::MethodKind::#kind,
                request_type: <#req_ty as ::by_grpc::prost::Name>::full_name,
                response_type: <#resp_ty as ::by_grpc::prost::Name>::full_name,
                handler: #handler_name,
                source: ::core::concat!(::core::module_path!(), "::", ::core::stringify!(#fn_name)),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_buf_version_suffixes() {
        for ok in ["v1", "v2", "v10", "v1alpha1", "v2beta3"] {
            assert!(is_valid_version(ok), "{ok}");
        }
        for bad in ["1", "v0", "v01", "v1alpha", "v1rc1", "V1", "v"] {
            assert!(!is_valid_version(bad), "{bad}");
        }
    }

    fn kinds(sig: &str) -> (&'static str, &'static str) {
        let f: ItemFn = syn::parse_str(&format!("{sig} {{}}")).unwrap();
        let FnArg::Typed(req) = f.sig.inputs.last().unwrap() else {
            unreachable!()
        };
        let input = match classify_input(&req.ty).unwrap() {
            Input::Message(_) => "message",
            Input::Stream(_) => "stream",
        };
        let ok = result_ok_type(&f.sig.output).unwrap();
        let output = match classify_output(ok).unwrap() {
            Output::Message(_) => "message",
            Output::Stream(_) => "stream",
        };
        (input, output)
    }

    #[test]
    fn classifies_the_four_rpc_shapes() {
        assert_eq!(
            kinds("async fn f(r: Req) -> Result<Reply<Resp>, E>"),
            ("message", "message")
        );
        assert_eq!(
            kinds("async fn f(r: Req) -> Result<impl Stream<Item = Result<Resp, Status>> + Send + 'static, E>"),
            ("message", "stream")
        );
        assert_eq!(
            kinds("async fn f(r: Streaming<Req>) -> Result<Resp, E>"),
            ("stream", "message")
        );
        assert_eq!(
            kinds("async fn f(r: impl Stream<Item = Result<Req, Status>>) -> Result<Reply<ResponseStream<Resp>>, E>"),
            ("stream", "stream")
        );
    }
}
