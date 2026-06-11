use dioxus_gtag::GtagEvent;

#[derive(GtagEvent)]
#[gtag(name = "purchase")]
struct Purchase {
    value: f64,
    currency: String,
    #[gtag(rename = "item_id")]
    sku: String,
    #[gtag(skip)]
    internal: bool,
}

#[derive(GtagEvent)]
struct QuizSubmitted {
    score: i64,
}

#[derive(GtagEvent)]
struct SignOut;

#[test]
fn derive_with_name_rename_and_skip() {
    let event = Purchase {
        value: 12000.0,
        currency: "KRW".to_string(),
        sku: "SKU_1".to_string(),
        internal: true,
    };
    assert_eq!(event.event_name(), "purchase");
    assert_eq!(
        event.params(),
        serde_json::json!({ "value": 12000.0, "currency": "KRW", "item_id": "SKU_1" })
    );
}

#[test]
fn event_name_defaults_to_snake_case() {
    let event = QuizSubmitted { score: 42 };
    assert_eq!(event.event_name(), "quiz_submitted");
    assert_eq!(event.params(), serde_json::json!({ "score": 42 }));
}

#[test]
fn unit_struct_has_empty_params() {
    let event = SignOut;
    assert_eq!(event.event_name(), "sign_out");
    assert_eq!(event.params(), serde_json::json!({}));
}
