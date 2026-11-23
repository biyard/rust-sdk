//! Runtime for `#[by_grpc::grpc]`.
//!
//! Each `#[grpc(service = "...", version = "...")]` function is registered as one
//! [`GrpcMethod`] through [`inventory`]. [`routes`] collects every registered method into
//! [`tonic::service::Routes`] keyed by the gRPC path (`/<package>.<version>.<Service>/<Method>`),
//! so a service crate no longer implements one tonic server trait on a single struct.
//!
//! ```ignore
//! use by_grpc::{grpc, Caller, Reply, Streaming};
//!
//! #[grpc(service = "biyard.greeter.Greeter", version = "v1")]
//! pub async fn say_hello(caller: Caller, req: HelloRequest) -> Result<HelloReply, Error> { .. }
//!
//! #[grpc(service = "biyard.greeter.Greeter", version = "v1")]
//! pub async fn count(req: CountRequest)
//!     -> Result<impl Stream<Item = Result<Tick, Status>> + Send + 'static, Error> { .. }
//!
//! #[grpc(service = "biyard.greeter.Greeter", version = "v1")]
//! pub async fn sum(numbers: Streaming<Number>) -> Result<Total, Error> { .. }
//!
//! by_grpc::verify(&[DESCRIPTOR_SET])?;
//! tonic::transport::Server::builder().add_routes(by_grpc::routes()?).serve(addr).await?;
//! ```
//!
//! The call context (`biyard.context.CallContext`, crate `call-context` in biyard/rbdk) and
//! response cookie directives travel as binary metadata today ([`CONTEXT_METADATA_KEY`],
//! [`COOKIE_METADATA_KEY`]). Handlers see them only through [`FromCall`] extractors and
//! [`Reply`], and callers only through [`client`], so the wire location can change inside
//! this crate alone.

pub mod client;
mod context;
mod method;
mod reply;
mod router;
mod transport;
mod verify;

pub use by_grpc_macros::grpc;
pub use context::{
    cookie_directive, CallContext, CallParts, Caller, ClearCookie, CookieDirective,
    CookieDirectives, DataRoomRole, DataRoomScope, FromCall, SetCookie, UserType, WorkspaceRole,
    WorkspaceScope,
};
pub use method::{
    bidi, client_streaming, server_streaming, unary, GrpcMethod, Handler, HandlerFuture, MethodKind,
};
pub use reply::{IntoReply, IntoStreamReply, Reply, ResponseStream};
pub use router::{routes, serve, RegistryError};
pub use tonic::{Status, Streaming};
pub use transport::{CONTEXT_METADATA_KEY, COOKIE_METADATA_KEY};
pub use verify::{verify, Mismatch};

pub use axum;
pub use futures_core;
pub use http;
pub use inventory;
pub use prost;
pub use tonic;

inventory::collect!(GrpcMethod);

/// Every method registered by `#[grpc]` in the final binary.
pub fn methods() -> impl Iterator<Item = &'static GrpcMethod> {
    inventory::iter::<GrpcMethod>.into_iter()
}
