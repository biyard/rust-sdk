mod common;

use example_grpc_server::proto::v1;
use tokio_stream::StreamExt;
use tonic::Code;

#[tokio::test]
async fn server_streaming_countdown() {
    let response = common::v1_client()
        .await
        .countdown(v1::CountdownRequest { from: 3 })
        .await
        .unwrap();
    assert_eq!(response.metadata().get("x-countdown-from").unwrap(), "3");
    let ticks: Vec<u32> = response
        .into_inner()
        .map(|tick| tick.unwrap().remaining)
        .collect()
        .await;
    assert_eq!(ticks, vec![3, 2, 1]);
}

#[tokio::test]
async fn server_streaming_error_before_the_stream() {
    let status = common::v1_client()
        .await
        .countdown(v1::CountdownRequest { from: 11 })
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
}

#[tokio::test]
async fn client_streaming_sum_with_context() {
    let numbers = tokio_stream::iter([1, 2, 39].map(|value| v1::Number { value }));
    let reply = common::v1_client()
        .await
        .sum(common::with_context(numbers, &common::signed_in()))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(reply.total, 42);
    assert_eq!(reply.count, 3);
    assert_eq!(reply.summed_for, "u-123");
}

#[tokio::test]
async fn bidi_chat_reads_context_once_per_stream() {
    let messages = tokio_stream::iter(["hi", "there"].map(|text| v1::ChatMessage {
        from: String::new(),
        text: text.into(),
    }));
    let replies: Vec<v1::ChatMessage> = common::v1_client()
        .await
        .chat(common::with_context(messages, &common::signed_in()))
        .await
        .unwrap()
        .into_inner()
        .map(|m| m.unwrap())
        .collect()
        .await;
    assert_eq!(
        replies,
        vec![
            v1::ChatMessage {
                from: "u-123".into(),
                text: "echo: hi".into(),
            },
            v1::ChatMessage {
                from: "u-123".into(),
                text: "echo: there".into(),
            },
        ]
    );
}

#[tokio::test]
async fn bidi_chat_requires_a_caller() {
    let messages = tokio_stream::iter([v1::ChatMessage::default()]);
    let status = common::v1_client().await.chat(messages).await.unwrap_err();
    assert_eq!(status.code(), Code::Unauthenticated);
}
