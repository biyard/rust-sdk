use by_grpc::{grpc, Status};

pub struct Req;
pub struct Resp;

#[grpc(service = "biyard.ui.Echo", version = "1")]
pub async fn echo(req: Req) -> Result<Resp, Status> {
    let _ = req;
    Ok(Resp)
}

fn main() {}
