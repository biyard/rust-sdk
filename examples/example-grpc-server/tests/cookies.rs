mod common;

use by_grpc::cookie_directive::Action;
use by_grpc::{ClearCookie, SetCookie};
use example_grpc_server::proto::v1;

#[tokio::test]
async fn sign_in_sets_the_session_cookie() {
    let response = common::v1_client()
        .await
        .sign_in(v1::SignInRequest {
            user_id: "u-123".into(),
        })
        .await
        .unwrap();
    let cookies = by_grpc::client::cookies(&response).unwrap();
    assert_eq!(cookies.len(), 1);
    assert_eq!(cookies[0].name, "sid");
    assert_eq!(
        cookies[0].action,
        Some(Action::Set(SetCookie {
            value: "session-u-123".into(),
            expires_at: 0,
        }))
    );
}

#[tokio::test]
async fn sign_out_reads_the_forwarded_cookie_and_clears_it() {
    let mut context = common::signed_in();
    context.cookies.insert("sid".into(), "session-u-123".into());
    let response = common::v1_client()
        .await
        .sign_out(common::with_context(v1::SignOutRequest {}, &context))
        .await
        .unwrap();
    let cookies = by_grpc::client::cookies(&response).unwrap();
    assert!(response.get_ref().had_session);
    assert_eq!(cookies.len(), 1);
    assert_eq!(cookies[0].action, Some(Action::Clear(ClearCookie {})));
}

#[tokio::test]
async fn plain_message_reply_carries_no_cookies() {
    let response = common::v1_client()
        .await
        .say_hello(v1::HelloRequest {
            name: "Miner".into(),
        })
        .await
        .unwrap();
    assert!(by_grpc::client::cookies(&response).unwrap().is_empty());
}
