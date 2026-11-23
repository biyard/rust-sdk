use by_grpc::{Caller, ResponseStream, Status, grpc};
use tokio_stream::{Stream, StreamExt};

use crate::proto::v1::ChatMessage;

/// `biyard.greeter.v1.Greeter/Chat`: bidirectional streaming. The caller comes from the
/// call context once, when the stream opens; every incoming message is echoed back as sent
/// by that caller.
#[grpc(service = "biyard.greeter.Greeter", version = "v1")]
pub async fn chat(
    caller: Caller,
    messages: impl Stream<Item = Result<ChatMessage, Status>> + Send + 'static,
) -> Result<ResponseStream<ChatMessage>, Status> {
    let from = caller.user_id;
    Ok(Box::pin(messages.map(move |message| {
        message.map(|m| ChatMessage {
            from: from.clone(),
            text: format!("echo: {}", m.text),
        })
    })))
}
