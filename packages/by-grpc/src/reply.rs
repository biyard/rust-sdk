use std::pin::Pin;

use futures_core::Stream;
use tonic::metadata::MetadataMap;
use tonic::Status;

use crate::context::CookieDirective;

/// Boxed response stream of a server-streaming or bidirectional `#[grpc]` function.
pub type ResponseStream<M> = Pin<Box<dyn Stream<Item = Result<M, Status>> + Send + 'static>>;

/// Response body plus response metadata and cookie directives for the BFF.
///
/// `T` is the response message for unary and client-streaming methods, and the response
/// stream for server-streaming and bidirectional methods. Metadata and cookies go out with
/// the response headers.
pub struct Reply<T> {
    pub body: T,
    pub metadata: MetadataMap,
    pub cookies: Vec<CookieDirective>,
}

impl<T> Reply<T> {
    pub fn new(body: T) -> Self {
        Self {
            body,
            metadata: MetadataMap::new(),
            cookies: Vec::new(),
        }
    }

    /// Asks the BFF to set `name`; `expires_at` is Unix seconds, `0` for a session cookie.
    pub fn set_cookie(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        expires_at: i64,
    ) -> Self {
        self.cookies
            .push(CookieDirective::set(name, value, expires_at));
        self
    }

    /// Asks the BFF to remove `name`.
    pub fn clear_cookie(mut self, name: impl Into<String>) -> Self {
        self.cookies.push(CookieDirective::clear(name));
        self
    }

    fn into_response<B>(self, map: impl FnOnce(T) -> B) -> tonic::Response<B> {
        let mut response = tonic::Response::new(map(self.body));
        *response.metadata_mut() = self.metadata;
        crate::transport::write_cookies(self.cookies, response.metadata_mut());
        response
    }
}

/// Ok-type of a unary or client-streaming `#[grpc]` function: `M` or `Reply<M>`.
pub trait IntoReply {
    type Message: prost::Message + prost::Name + Send + 'static;

    fn into_reply(self) -> tonic::Response<Self::Message>;
}

impl<M: prost::Message + prost::Name + Send + 'static> IntoReply for M {
    type Message = M;

    fn into_reply(self) -> tonic::Response<M> {
        tonic::Response::new(self)
    }
}

impl<M: prost::Message + prost::Name + Send + 'static> IntoReply for Reply<M> {
    type Message = M;

    fn into_reply(self) -> tonic::Response<M> {
        self.into_response(|m| m)
    }
}

/// Ok-type of a server-streaming or bidirectional `#[grpc]` function: a stream of
/// `Result<M, Status>` or `Reply` of one.
pub trait IntoStreamReply<M> {
    fn into_stream_reply(self) -> tonic::Response<ResponseStream<M>>;
}

impl<M, S> IntoStreamReply<M> for S
where
    S: Stream<Item = Result<M, Status>> + Send + 'static,
{
    fn into_stream_reply(self) -> tonic::Response<ResponseStream<M>> {
        tonic::Response::new(Box::pin(self))
    }
}

impl<M, S> IntoStreamReply<M> for Reply<S>
where
    S: Stream<Item = Result<M, Status>> + Send + 'static,
{
    fn into_stream_reply(self) -> tonic::Response<ResponseStream<M>> {
        self.into_response(|s| Box::pin(s) as ResponseStream<M>)
    }
}
