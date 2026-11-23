use by_grpc::{CallContext, Reply, grpc};
use tonic::metadata::MetadataValue;

use crate::Error;
use crate::proto::v2::{HelloReply, HelloRequest};

/// `biyard.greeter.v2.Greeter/SayHello`: localized greeting that reports the caller and
/// tags the response metadata with the served version.
#[grpc(
    service = "biyard.greeter.Greeter",
    version = "v2",
    method = "SayHello"
)]
pub async fn say_hello_v2(ctx: CallContext, req: HelloRequest) -> Result<Reply<HelloReply>, Error> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(Error::NameRequired);
    }
    let greeting = match req.locale.as_str() {
        "ko" => "안녕하세요",
        _ => "Hello",
    };
    let greeted_by = ctx
        .caller
        .map(|caller| caller.user_id)
        .unwrap_or_else(|| "anonymous".to_string());
    let mut reply = Reply::new(HelloReply {
        message: format!("{greeting}, {name}"),
        greeted_by,
    });
    reply
        .metadata
        .insert("x-greeter-version", MetadataValue::from_static("v2"));
    Ok(reply)
}
