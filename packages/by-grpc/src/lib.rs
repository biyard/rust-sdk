//! Runtime for `#[by_macros::grpc]`.
//!
//! Each `#[grpc(service = "...", version = "...")]` function is registered as one
//! [`GrpcMethod`] through [`inventory`]. [`routes`] collects every registered method into
//! a [`tonic::service::Routes`] keyed by the gRPC path (`/<package>.<version>.<Service>/<Method>`),
//! so a service crate no longer implements one tonic server trait on a single struct.
//!
//! ```ignore
//! #[grpc(service = "biyard.greeter.Greeter", version = "v1")]
//! pub async fn say_hello(caller: Caller, req: HelloRequest) -> Result<HelloReply, Error> { .. }
//!
//! by_grpc::verify(&[DESCRIPTOR_SET])?;
//! tonic::transport::Server::builder().add_routes(by_grpc::routes()?).serve(addr).await?;
//! ```

mod context;
mod method;
mod reply;
mod router;
mod verify;

pub use context::{
    CallContext, CallParts, Caller, DataRoomScope, FromCall, WorkspaceScope, CONTEXT_METADATA_KEY,
};
pub use method::{unary, GrpcMethod, Handler, HandlerFuture};
pub use reply::{IntoReply, Reply};
pub use router::{routes, serve, RegistryError};
pub use verify::{verify, Mismatch};

pub use axum;
pub use http;
pub use inventory;
pub use prost;
pub use tonic;

inventory::collect!(GrpcMethod);

/// Every method registered by `#[grpc]` in the final binary.
pub fn methods() -> impl Iterator<Item = &'static GrpcMethod> {
    inventory::iter::<GrpcMethod>.into_iter()
}
