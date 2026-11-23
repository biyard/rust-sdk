mod common;

use example_grpc_server::proto::{v1, v2};
use tonic::Code;

#[tokio::test]
async fn v1_and_v2_say_hello_coexist() {
    let ch = common::channel().await;

    let v1_reply = v1::greeter_client::GreeterClient::new(ch.clone())
        .say_hello(v1::HelloRequest {
            name: "Miner".into(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(v1_reply.message, "Hello, Miner");

    let request = common::with_context(
        v2::HelloRequest {
            name: "Miner".into(),
            locale: "ko".into(),
        },
        &common::signed_in(),
    );
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
async fn handler_error_becomes_status() {
    let status = common::v1_client()
        .await
        .say_hello(v1::HelloRequest { name: " ".into() })
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
    assert_eq!(status.message(), "name is required");
}

#[tokio::test]
async fn unregistered_version_is_unimplemented() {
    let ch = common::channel().await;
    let mut grpc = tonic::client::Grpc::new(ch);
    grpc.ready().await.unwrap();
    let status = grpc
        .unary::<_, v1::HelloReply, _>(
            tonic::Request::new(v1::HelloRequest {
                name: "Miner".into(),
            }),
            "/biyard.greeter.v3.Greeter/SayHello".parse().unwrap(),
            tonic_prost::ProstCodec::default(),
        )
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::Unimplemented);
}

#[tokio::test]
async fn invalid_context_metadata_is_rejected() {
    let mut request = tonic::Request::new(v1::HelloRequest {
        name: "Miner".into(),
    });
    request.metadata_mut().insert_bin(
        by_grpc::CONTEXT_METADATA_KEY,
        tonic::metadata::MetadataValue::from_bytes(&[0xff, 0xff]),
    );
    let status = common::v1_client()
        .await
        .say_hello(request)
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
}
