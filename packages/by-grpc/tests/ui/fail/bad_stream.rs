use by_grpc::{grpc, Status};
use tokio_stream::Stream;

pub struct Req;
pub struct Resp;

#[grpc(service = "biyard.ui.Echo", version = "v1")]
pub async fn item_not_result(req: Req) -> Result<impl Stream<Item = Resp> + Send + 'static, Status> {
    let _ = req;
    Ok(tokio_stream::iter([Resp]))
}

#[grpc(service = "biyard.ui.Echo", version = "v1")]
pub async fn request_not_stream(
    req: impl Iterator<Item = Req>,
) -> Result<Resp, Status> {
    let _ = req;
    Ok(Resp)
}

fn main() {}
