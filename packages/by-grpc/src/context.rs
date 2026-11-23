use std::collections::HashMap;
use std::future::Future;

use tonic::metadata::{MetadataMap, MetadataValue};
use tonic::{Extensions, Status};

/// Binary metadata key that carries the protobuf-encoded [`CallContext`].
pub const CONTEXT_METADATA_KEY: &str = "biyard-context-bin";

/// Authenticated user forwarded by the BFF (`biyard.context.Caller`).
#[derive(Clone, PartialEq, prost::Message)]
pub struct Caller {
    #[prost(string, tag = "1")]
    pub user_id: String,
    #[prost(int32, tag = "2")]
    pub user_type: i32,
}

/// Workspace the call targets (`biyard.context.WorkspaceScope`).
#[derive(Clone, PartialEq, prost::Message)]
pub struct WorkspaceScope {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(int32, optional, tag = "3")]
    pub role: Option<i32>,
}

/// Data room the call targets (`biyard.context.DataRoomScope`).
#[derive(Clone, PartialEq, prost::Message)]
pub struct DataRoomScope {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(int32, tag = "2")]
    pub role: i32,
}

/// Call context the BFF attaches to every request.
///
/// Same fields and tags as `biyard.context.RpcCall` minus `params`, so the BFF can move
/// the existing envelope fields into metadata without re-mapping them.
#[derive(Clone, PartialEq, prost::Message)]
pub struct CallContext {
    #[prost(message, optional, tag = "9")]
    pub caller: Option<Caller>,
    #[prost(message, optional, tag = "10")]
    pub workspace: Option<WorkspaceScope>,
    #[prost(message, optional, tag = "11")]
    pub dataroom: Option<DataRoomScope>,
    #[prost(map = "string, string", tag = "12")]
    pub cookies: HashMap<String, String>,
}

impl CallContext {
    /// Decodes the context from request metadata; an absent key yields an empty context.
    pub fn from_metadata(metadata: &MetadataMap) -> Result<Self, Status> {
        let Some(value) = metadata.get_bin(CONTEXT_METADATA_KEY) else {
            return Ok(Self::default());
        };
        let bytes = value
            .to_bytes()
            .map_err(|_| Status::invalid_argument("call context is not valid base64"))?;
        <Self as prost::Message>::decode(bytes)
            .map_err(|_| Status::invalid_argument("call context is not a valid message"))
    }

    /// Writes the context into outgoing metadata (BFF and tests).
    pub fn insert_into(&self, metadata: &mut MetadataMap) {
        let bytes = prost::Message::encode_to_vec(self);
        metadata.insert_bin(CONTEXT_METADATA_KEY, MetadataValue::from_bytes(&bytes));
    }
}

/// Request parts handed to every [`FromCall`] extractor before the handler runs.
pub struct CallParts {
    pub metadata: MetadataMap,
    pub extensions: Extensions,
    pub context: CallContext,
}

/// Builds a handler argument from the call, like axum's `FromRequestParts`.
///
/// Every `#[grpc]` parameter except the last (the request message) must implement this.
/// Returning `Err` short-circuits the call with that status.
pub trait FromCall: Sized {
    fn from_call(parts: &CallParts) -> impl Future<Output = Result<Self, Status>> + Send;
}

impl FromCall for CallContext {
    async fn from_call(parts: &CallParts) -> Result<Self, Status> {
        Ok(parts.context.clone())
    }
}

impl FromCall for MetadataMap {
    async fn from_call(parts: &CallParts) -> Result<Self, Status> {
        Ok(parts.metadata.clone())
    }
}

impl<T: FromCall> FromCall for Option<T> {
    async fn from_call(parts: &CallParts) -> Result<Self, Status> {
        Ok(T::from_call(parts).await.ok())
    }
}
