//! `#[grpc]` attribute for `by-grpc`. Use it through `by_grpc::grpc`; the generated code
//! refers to `::by_grpc`.

use proc_macro::TokenStream;

mod grpc;

/// Registers an async fn as one gRPC method served by `by_grpc::routes()`.
///
/// ```ignore
/// #[grpc(service = "biyard.greeter.Greeter", version = "v1")]
/// pub async fn say_hello(caller: Caller, req: HelloRequest) -> Result<HelloReply, Error> { .. }
/// ```
///
/// - `service`: `<package>.<Service>` without the version. `version` (required): `v1`, `v2`,
///   `v1alpha1`, `v1beta1`. Route: `/<package>.<version>.<Service>/<Method>`; `method`
///   defaults to the PascalCase fn name.
/// - Every parameter but the last implements `by_grpc::FromCall`.
/// - Last parameter: the prost request message, or for client streaming / bidi
///   `by_grpc::Streaming<Req>` or `impl Stream<Item = Result<Req, Status>>`.
/// - Ok type: `M` or `Reply<M>` for unary / client streaming; for server streaming / bidi
///   `impl Stream<Item = Result<M, Status>> + Send + 'static`, `ResponseStream<M>`, or `Reply`
///   of either. `M: prost::Name`; the error type implements `Into<tonic::Status>`.
#[proc_macro_attribute]
pub fn grpc(attr: TokenStream, item: TokenStream) -> TokenStream {
    grpc::grpc_impl(attr.into(), item.into()).into()
}
