//! Example service built on `#[by_macros::grpc]`: each RPC is a free function in its own
//! module, registered through `inventory`, with `greeter.v1` and `greeter.v2` served side by side.

pub mod auth;
pub mod error;
pub mod methods;

pub mod proto {
    pub mod v1 {
        tonic::include_proto!("biyard.greeter.v1");
    }
    pub mod v2 {
        tonic::include_proto!("biyard.greeter.v2");
    }
}

/// `FileDescriptorSet` of every proto this service owns, for `by_grpc::verify`.
pub const DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("greeter_descriptor");

pub use error::Error;
