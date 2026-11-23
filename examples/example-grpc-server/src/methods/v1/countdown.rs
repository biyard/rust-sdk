use by_grpc::{Reply, Status, grpc};
use tokio_stream::Stream;
use tonic::metadata::MetadataValue;

use crate::Error;
use crate::proto::v1::{CountdownRequest, Tick};

/// Longest countdown `Countdown` accepts.
pub const MAX_COUNTDOWN: u32 = 10;

/// `biyard.greeter.v1.Greeter/Countdown`: server streaming. Streams `from, from-1, .., 1`;
/// an invalid request fails before the stream starts.
#[grpc(service = "biyard.greeter.Greeter", version = "v1")]
pub async fn countdown(
    req: CountdownRequest,
) -> Result<Reply<impl Stream<Item = Result<Tick, Status>> + Send + 'static>, Error> {
    if req.from > MAX_COUNTDOWN {
        return Err(Error::CountdownTooLong { max: MAX_COUNTDOWN });
    }
    let ticks = (1..=req.from).rev().map(|remaining| Ok(Tick { remaining }));
    let mut reply = Reply::new(tokio_stream::iter(ticks));
    reply
        .metadata
        .insert("x-countdown-from", MetadataValue::from(req.from));
    Ok(reply)
}
