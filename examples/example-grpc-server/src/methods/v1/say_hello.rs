use by_grpc::grpc;

use crate::Error;
use crate::proto::v1::{HelloReply, HelloRequest};

/// `biyard.greeter.v1.Greeter/SayHello`.
#[grpc(service = "biyard.greeter.Greeter", version = "v1")]
pub async fn say_hello(req: HelloRequest) -> Result<HelloReply, Error> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(Error::NameRequired);
    }
    Ok(HelloReply {
        message: format!("Hello, {name}"),
    })
}
