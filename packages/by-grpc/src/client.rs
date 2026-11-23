//! Caller-side helpers for the BFF and tests: attach the call context to a request and read
//! the cookie directives off a response, wherever [`crate`] decides they travel.

use tonic::Status;

use crate::context::{CallContext, CookieDirective};

/// Attaches `context` to an outgoing request.
pub fn attach_context<T>(request: &mut tonic::Request<T>, context: &CallContext) {
    crate::transport::write_context(context, request.metadata_mut());
}

/// Cookie directives the service returned; empty when it returned none.
pub fn cookies<T>(response: &tonic::Response<T>) -> Result<Vec<CookieDirective>, Status> {
    crate::transport::read_cookies(response.metadata())
}
