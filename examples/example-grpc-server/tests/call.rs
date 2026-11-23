use std::net::SocketAddr;

use by_grpc::{CallContext, Caller, Mismatch, WorkspaceScope};
use grpc_macro_server::proto::{v1, v2};
use prost::Message;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::Code;
use tonic::transport::Channel;

async fn spawn_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let routes = by_grpc::routes().expect("registry builds");
    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_routes(routes)
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });
    addr
}

async fn channel(addr: SocketAddr) -> Channel {
    Channel::from_shared(format!("http://{addr}"))
        .unwrap()
        .connect()
        .await
        .unwrap()
}

fn signed_in_context() -> CallContext {
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

#[tokio::test]
async fn v1_and_v2_say_hello_coexist() {
    let ch = channel(spawn_server().await).await;

    let v1_reply = v1::greeter_client::GreeterClient::new(ch.clone())
        .say_hello(v1::HelloRequest {
            name: "Miner".into(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(v1_reply.message, "Hello, Miner");

    let mut request = tonic::Request::new(v2::HelloRequest {
        name: "Miner".into(),
        locale: "ko".into(),
    });
    signed_in_context().insert_into(request.metadata_mut());
    let v2_response = v2::greeter_client::GreeterClient::new(ch)
        .say_hello(request)
        .await
        .unwrap();
    assert_eq!(
        v2_response.metadata().get("x-greeter-version").unwrap(),
        "v2"
    );
    let v2_reply = v2_response.into_inner();
    assert_eq!(v2_reply.message, "안녕하세요, Miner");
    assert_eq!(v2_reply.greeted_by, "u-123");
}

#[tokio::test]
async fn handler_errors_map_to_status() {
    let ch = channel(spawn_server().await).await;
    let status = v1::greeter_client::GreeterClient::new(ch)
        .say_hello(v1::HelloRequest { name: " ".into() })
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
}

#[tokio::test]
async fn extractor_rejects_missing_caller() {
    let ch = channel(spawn_server().await).await;
    let status = v1::greeter_client::GreeterClient::new(ch)
        .who_am_i(v1::WhoAmIRequest {})
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::Unauthenticated);
}

#[tokio::test]
async fn extractors_read_call_context() {
    let ch = channel(spawn_server().await).await;
    let mut request = tonic::Request::new(v1::WhoAmIRequest {});
    signed_in_context().insert_into(request.metadata_mut());
    let reply = v1::greeter_client::GreeterClient::new(ch)
        .who_am_i(request)
        .await
        .unwrap()
        .into_inner();
    assert_eq!(reply.user_id, "u-123");
    assert_eq!(reply.workspace_id.as_deref(), Some("ws-1"));
}

#[tokio::test]
async fn unregistered_version_is_unimplemented() {
    let ch = channel(spawn_server().await).await;
    let mut grpc = tonic::client::Grpc::new(ch);
    grpc.ready().await.unwrap();
    let status = grpc
        .unary::<_, v1::HelloReply, _>(
            tonic::Request::new(v1::HelloRequest {
                name: "Miner".into(),
            }),
            "/biyard.greeter.v3.Greeter/SayHello".parse().unwrap(),
            tonic::codec::ProstCodec::default(),
        )
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::Unimplemented);
}

#[test]
fn registry_matches_descriptors() {
    assert_eq!(by_grpc::verify(&[grpc_macro_server::DESCRIPTOR_SET]), Ok(()));
}

#[test]
fn verify_reports_proto_drift() {
    let mut set =
        prost_types::FileDescriptorSet::decode(grpc_macro_server::DESCRIPTOR_SET).unwrap();
    let v1_file = set
        .file
        .iter_mut()
        .find(|f| f.package() == "biyard.greeter.v1")
        .unwrap();
    let service = &mut v1_file.service[0];
    let mut goodbye = service.method[0].clone();
    goodbye.name = Some("Goodbye".into());
    service.method.push(goodbye);
    service.method[0].input_type = Some(".biyard.greeter.v1.WhoAmIRequest".into());

    let drifted = set.encode_to_vec();
    let mismatches = by_grpc::verify(&[&drifted]).unwrap_err();
    assert!(mismatches.contains(&Mismatch::Unregistered {
        path: "/biyard.greeter.v1.Greeter/Goodbye".into()
    }));
    assert!(mismatches.contains(&Mismatch::RequestType {
        path: "/biyard.greeter.v1.Greeter/SayHello".into(),
        proto: "biyard.greeter.v1.WhoAmIRequest".into(),
        rust: "biyard.greeter.v1.HelloRequest".into(),
    }));
}
