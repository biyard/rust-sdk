use by_grpc::{grpc, Caller, Reply, ResponseStream, Status, Streaming};
use tokio_stream::{Stream, StreamExt};

#[derive(Clone, PartialEq, prost::Message)]
pub struct Req {
    #[prost(string, tag = "1")]
    pub text: String,
}

impl prost::Name for Req {
    const NAME: &'static str = "Req";
    const PACKAGE: &'static str = "biyard.ui.v1";
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Resp {
    #[prost(string, tag = "1")]
    pub text: String,
}

impl prost::Name for Resp {
    const NAME: &'static str = "Resp";
    const PACKAGE: &'static str = "biyard.ui.v1";
}

#[grpc(service = "biyard.ui.Echo", version = "v1")]
pub async fn unary(caller: Option<Caller>, req: Req) -> Result<Reply<Resp>, Status> {
    Ok(Reply::new(Resp { text: req.text }).set_cookie("seen", caller.is_some().to_string(), 0))
}

#[grpc(service = "biyard.ui.Echo", version = "v1")]
pub async fn server_streaming(
    req: Req,
) -> Result<impl Stream<Item = Result<Resp, Status>> + Send + 'static, Status> {
    Ok(tokio_stream::iter([Ok(Resp { text: req.text })]))
}

#[grpc(service = "biyard.ui.Echo", version = "v1")]
pub async fn client_streaming(mut reqs: Streaming<Req>) -> Result<Resp, Status> {
    let mut text = String::new();
    while let Some(req) = reqs.message().await? {
        text.push_str(&req.text);
    }
    Ok(Resp { text })
}

#[grpc(service = "biyard.ui.Echo", version = "v1beta1", method = "Bidi")]
pub async fn bidi_beta(
    reqs: impl Stream<Item = Result<Req, Status>> + Send + 'static,
) -> Result<Reply<ResponseStream<Resp>>, Status> {
    Ok(Reply::new(Box::pin(
        reqs.map(|r| r.map(|r| Resp { text: r.text })),
    )))
}

fn main() {
    let mut paths: Vec<_> = by_grpc::methods()
        .map(|m| (m.path, m.kind))
        .collect();
    paths.sort_by_key(|(path, _)| *path);
    assert_eq!(
        paths,
        vec![
            ("/biyard.ui.v1.Echo/ClientStreaming", by_grpc::MethodKind::ClientStreaming),
            ("/biyard.ui.v1.Echo/ServerStreaming", by_grpc::MethodKind::ServerStreaming),
            ("/biyard.ui.v1.Echo/Unary", by_grpc::MethodKind::Unary),
            ("/biyard.ui.v1beta1.Echo/Bidi", by_grpc::MethodKind::Bidi),
        ]
    );
}
