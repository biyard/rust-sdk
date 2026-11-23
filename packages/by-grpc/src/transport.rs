//! Wire location of the call context and the cookie directives.
//!
//! Only this module knows that both travel as binary metadata. Moving them into request or
//! response message fields changes this module and nothing that uses `by-grpc`.

use tonic::metadata::{MetadataMap, MetadataValue};
use tonic::Status;

use crate::context::{CallContext, CookieDirective, CookieDirectives};

/// Request metadata key carrying the protobuf-encoded `biyard.context.CallContext`.
pub const CONTEXT_METADATA_KEY: &str = "biyard-context-bin";

/// Response metadata key carrying the protobuf-encoded `biyard.context.CookieDirectives`.
pub const COOKIE_METADATA_KEY: &str = "biyard-cookie-bin";

pub(crate) fn read_context(metadata: &MetadataMap) -> Result<CallContext, Status> {
    decode_bin(metadata, CONTEXT_METADATA_KEY, "call context")
}

pub(crate) fn write_context(context: &CallContext, metadata: &mut MetadataMap) {
    let bytes = prost::Message::encode_to_vec(context);
    metadata.insert_bin(CONTEXT_METADATA_KEY, MetadataValue::from_bytes(&bytes));
}

pub(crate) fn write_cookies(cookies: Vec<CookieDirective>, metadata: &mut MetadataMap) {
    if cookies.is_empty() {
        return;
    }
    let bytes = prost::Message::encode_to_vec(&CookieDirectives { cookies });
    metadata.insert_bin(COOKIE_METADATA_KEY, MetadataValue::from_bytes(&bytes));
}

pub(crate) fn read_cookies(metadata: &MetadataMap) -> Result<Vec<CookieDirective>, Status> {
    decode_bin::<CookieDirectives>(metadata, COOKIE_METADATA_KEY, "cookie directives")
        .map(|d| d.cookies)
}

fn decode_bin<M: prost::Message + Default>(
    metadata: &MetadataMap,
    key: &'static str,
    what: &'static str,
) -> Result<M, Status> {
    let Some(value) = metadata.get_bin(key) else {
        return Ok(M::default());
    };
    let bytes = value
        .to_bytes()
        .map_err(|_| Status::invalid_argument(format!("{what} is not valid base64")))?;
    M::decode(bytes).map_err(|_| Status::invalid_argument(format!("{what} is not a valid message")))
}
