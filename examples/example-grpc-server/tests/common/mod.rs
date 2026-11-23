#![allow(dead_code)]

use std::net::SocketAddr;

use by_grpc::{CallContext, Caller, WorkspaceScope};
use example_grpc_server::proto::v1::greeter_client::GreeterClient;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Channel;

/// Serves every `#[grpc]` method linked into the test binary on an ephemeral port.
pub async fn channel() -> Channel {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let routes = by_grpc::routes().expect("registry builds");
    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_routes(routes)
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });
    Channel::from_shared(format!("http://{addr}"))
        .unwrap()
        .connect()
        .await
        .unwrap()
}

pub async fn v1_client() -> GreeterClient<Channel> {
    GreeterClient::new(channel().await)
}

pub fn signed_in() -> CallContext {
    CallContext {
        caller: Some(Caller {
            user_id: "u-123".into(),
            user_type: 0,
        }),
        workspace: Some(WorkspaceScope {
            id: "ws-1".into(),
            role: None,
        }),
        ..Default::default()
    }
}

/// `message` wrapped in a request that carries `context`.
pub fn with_context<T>(message: T, context: &CallContext) -> tonic::Request<T> {
    let mut request = tonic::Request::new(message);
    by_grpc::client::attach_context(&mut request, context);
    request
}
