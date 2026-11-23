use by_grpc::{Caller, Status, Streaming, grpc};

use crate::proto::v1::{Number, SumReply};

/// `biyard.greeter.v1.Greeter/Sum`: client streaming. Adds every number the client sends.
#[grpc(service = "biyard.greeter.Greeter", version = "v1")]
pub async fn sum(
    caller: Option<Caller>,
    mut numbers: Streaming<Number>,
) -> Result<SumReply, Status> {
    let mut total = 0i64;
    let mut count = 0u32;
    while let Some(number) = numbers.message().await? {
        total += number.value;
        count += 1;
    }
    Ok(SumReply {
        total,
        count,
        summed_for: caller.map(|c| c.user_id).unwrap_or_default(),
    })
}
