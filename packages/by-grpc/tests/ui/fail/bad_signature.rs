use by_grpc::{grpc, Status};

pub struct Req;
pub struct Resp;

#[grpc(service = "biyard.ui.Echo", version = "v1")]
pub fn echo(req: Req) -> Result<Resp, Status> {
    let _ = req;
    Ok(Resp)
}

#[grpc(service = "biyard.ui.Echo", version = "v1")]
pub async fn no_request() -> Result<Resp, Status> {
    Ok(Resp)
}

#[grpc(service = "biyard.ui.Echo", version = "v1")]
pub async fn not_result(req: Req) -> Resp {
    let _ = req;
    Resp
}

#[grpc(service = "biyard.ui.Echo", version = "v1")]
pub async fn generic<T>(req: T) -> Result<Resp, Status> {
    let _ = req;
    Ok(Resp)
}

#[grpc(service = "biyard.ui.Echo", version = "v1", path = "/x")]
pub async fn unknown_attribute(req: Req) -> Result<Resp, Status> {
    let _ = req;
    Ok(Resp)
}

fn main() {}
