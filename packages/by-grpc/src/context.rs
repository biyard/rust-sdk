use std::future::Future;

use tonic::metadata::MetadataMap;
use tonic::{Extensions, Status};

pub use call_context::{
    cookie_directive, CallContext, Caller, ClearCookie, CookieDirective, CookieDirectives,
    DataRoomRole, DataRoomScope, SetCookie, UserType, WorkspaceRole, WorkspaceScope,
};

/// Request parts handed to every [`FromCall`] extractor before the handler runs.
///
/// Built once per call; for streaming RPCs that is once per stream, from the initial headers.
pub struct CallParts {
    metadata: MetadataMap,
    extensions: Extensions,
    context: CallContext,
}

impl CallParts {
    pub(crate) fn new(metadata: MetadataMap, extensions: Extensions) -> Result<Self, Status> {
        let context = crate::transport::read_context(&metadata)?;
        Ok(Self {
            metadata,
            extensions,
            context,
        })
    }

    /// Call context the BFF attached; empty when it sent none.
    pub fn context(&self) -> &CallContext {
        &self.context
    }

    /// Raw request metadata.
    pub fn metadata(&self) -> &MetadataMap {
        &self.metadata
    }

    /// Request extensions set by tower layers in front of the server.
    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }
}

/// Builds a handler argument from the call, like axum's `FromRequestParts`.
///
/// Every `#[grpc]` parameter except the last (the request message or stream) implements this.
/// Returning `Err` ends the call with that status before the handler runs.
pub trait FromCall: Sized {
    fn from_call(parts: &CallParts) -> impl Future<Output = Result<Self, Status>> + Send;
}

impl FromCall for CallContext {
    async fn from_call(parts: &CallParts) -> Result<Self, Status> {
        Ok(parts.context.clone())
    }
}

/// Signed-in caller; `UNAUTHENTICATED` when the BFF sent none.
impl FromCall for Caller {
    async fn from_call(parts: &CallParts) -> Result<Self, Status> {
        parts
            .context
            .caller
            .clone()
            .ok_or_else(|| Status::unauthenticated("caller is required"))
    }
}

/// Target workspace; `INVALID_ARGUMENT` when the BFF sent none.
impl FromCall for WorkspaceScope {
    async fn from_call(parts: &CallParts) -> Result<Self, Status> {
        parts
            .context
            .workspace
            .clone()
            .ok_or_else(|| Status::invalid_argument("workspace scope is required"))
    }
}

/// Target data room; `INVALID_ARGUMENT` when the BFF sent none.
impl FromCall for DataRoomScope {
    async fn from_call(parts: &CallParts) -> Result<Self, Status> {
        parts
            .context
            .dataroom
            .clone()
            .ok_or_else(|| Status::invalid_argument("data room scope is required"))
    }
}

impl FromCall for MetadataMap {
    async fn from_call(parts: &CallParts) -> Result<Self, Status> {
        Ok(parts.metadata.clone())
    }
}

/// `None` instead of an error when the inner extractor rejects the call.
impl<T: FromCall> FromCall for Option<T> {
    async fn from_call(parts: &CallParts) -> Result<Self, Status> {
        Ok(T::from_call(parts).await.ok())
    }
}
