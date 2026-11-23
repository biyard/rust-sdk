use by_grpc::cookie_directive::Action;
use by_grpc::tonic;
use by_grpc::{CallContext, Caller, ClearCookie, IntoReply, Reply, SetCookie};

#[derive(Clone, PartialEq, prost::Message)]
struct Resp {
    #[prost(string, tag = "1")]
    text: String,
}

impl prost::Name for Resp {
    const NAME: &'static str = "Resp";
    const PACKAGE: &'static str = "biyard.test.v1";
}

#[test]
fn reply_cookies_round_trip_through_the_client_helper() {
    let response = Reply::new(Resp { text: "ok".into() })
        .set_cookie("sid", "abc", 99)
        .clear_cookie("old")
        .into_reply();
    let cookies = by_grpc::client::cookies(&response).unwrap();
    assert_eq!(cookies.len(), 2);
    assert_eq!(
        cookies[0].action,
        Some(Action::Set(SetCookie {
            value: "abc".into(),
            expires_at: 99,
        }))
    );
    assert_eq!(cookies[1].name, "old");
    assert_eq!(cookies[1].action, Some(Action::Clear(ClearCookie {})));
    assert_eq!(response.get_ref().text, "ok");
}

#[test]
fn attached_context_lands_in_metadata() {
    let context = CallContext {
        caller: Some(Caller {
            user_id: "u-1".into(),
            user_type: 0,
        }),
        ..Default::default()
    };
    let mut request = tonic::Request::new(());
    by_grpc::client::attach_context(&mut request, &context);
    let raw = request
        .metadata()
        .get_bin(by_grpc::CONTEXT_METADATA_KEY)
        .unwrap()
        .to_bytes()
        .unwrap();
    assert_eq!(
        <CallContext as prost::Message>::decode(raw).unwrap(),
        context
    );
}
