//! Example service on `#[by_grpc::grpc]`: every RPC is a free function in its own file,
//! registered through `inventory`, with `greeter.v1` and `greeter.v2` served side by side.
//!
//! | file | RPC | shows |
//! |---|---|---|
//! | `methods/v1/say_hello.rs` | `v1.SayHello` | unary, error → `Status` |
//! | `methods/v2/say_hello.rs` | `v2.SayHello` | same method in a newer package version, response metadata |
//! | `methods/v1/who_am_i.rs` | `v1.WhoAmI` | custom and optional context extractors |
//! | `methods/v1/sign_in.rs`, `sign_out.rs` | `v1.SignIn`, `v1.SignOut` | cookie directives, forwarded request cookies |
//! | `methods/v1/countdown.rs` | `v1.Countdown` | server streaming |
//! | `methods/v1/sum.rs` | `v1.Sum` | client streaming |
//! | `methods/v1/chat.rs` | `v1.Chat` | bidirectional streaming, context once per stream |

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
