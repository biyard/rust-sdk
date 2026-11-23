mod common;

use example_grpc_server::proto::v1;
use tonic::Code;

#[tokio::test]
async fn extractor_rejects_missing_caller() {
    let status = common::v1_client()
        .await
        .who_am_i(v1::WhoAmIRequest {})
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::Unauthenticated);
}

#[tokio::test]
async fn extractors_read_caller_and_optional_workspace() {
    let mut client = common::v1_client().await;

    let reply = client
        .who_am_i(common::with_context(
            v1::WhoAmIRequest {},
            &common::signed_in(),
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(reply.user_id, "u-123");
    assert_eq!(reply.workspace_id.as_deref(), Some("ws-1"));

    let mut no_workspace = common::signed_in();
    no_workspace.workspace = None;
    let reply = client
        .who_am_i(common::with_context(v1::WhoAmIRequest {}, &no_workspace))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(reply.workspace_id, None);
}
