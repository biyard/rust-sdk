use tonic::metadata::MetadataMap;

/// Ok-type of a `#[grpc]` function: a plain proto message or a [`Reply`] carrying metadata.
pub trait IntoReply {
    type Message: prost::Message + prost::Name + Send + 'static;

    fn into_reply(self) -> tonic::Response<Self::Message>;
}

impl<T: prost::Message + prost::Name + Send + 'static> IntoReply for T {
    type Message = T;

    fn into_reply(self) -> tonic::Response<T> {
        tonic::Response::new(self)
    }
}

/// Response message plus response metadata (e.g. cookie directives for the BFF).
pub struct Reply<T> {
    pub message: T,
    pub metadata: MetadataMap,
}

impl<T> Reply<T> {
    pub fn new(message: T) -> Self {
        Self {
            message,
            metadata: MetadataMap::new(),
        }
    }
}

impl<T: prost::Message + prost::Name + Send + 'static> IntoReply for Reply<T> {
    type Message = T;

    fn into_reply(self) -> tonic::Response<T> {
        let mut response = tonic::Response::new(self.message);
        *response.metadata_mut() = self.metadata;
        response
    }
}
