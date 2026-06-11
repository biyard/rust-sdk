# dioxus-gtag

Google Analytics 4 (gtag.js) integration for Dioxus.

- No `index.html` edits — the provider injects gtag.js via `document::eval`
- SSR/fullstack safe — initialization runs in an effect, so it only happens on the client
- Works on web and desktop (webview); no `#[cfg]` needed at call sites
- Events fired before gtag.js is ready are buffered and flushed on initialization
- Consent Mode v2, `user_id`/user properties, runtime opt-in/opt-out
- Optional `router` feature for automatic `page_view` on route changes

## Setup

```toml
[dependencies]
dioxus-gtag = { version = "0.1", features = ["router"] }
```

Provide the context once at the App root, like other context providers:

```rust
use dioxus::prelude::*;
use dioxus_gtag::prelude::*;

#[component]
pub fn App() -> Element {
    use_gtag_context_provider("G-XXXXXXXXXX");

    rsx! { Router::<Route> {} }
}
```

For more control, use `GtagConfig`:

```rust
use_gtag_context_provider_with(
    GtagConfig::new("G-XXXXXXXXXX")
        .debug(cfg!(debug_assertions))          // events show in GA4 DebugView
        .default_consent(ConsentUpdate::deny_all()) // Consent Mode v2 default
        .enabled(false),                        // opt-in flow: load nothing until consent
);
```

## Sending events

In a component or controller (hook context), use `use_gtag_context()`:

```rust
#[derive(Debug, Clone, Copy, DioxusController)]
pub struct Controller {
    gtag: UseGtagContext,
}

impl Controller {
    pub fn new(lang: Language) -> std::result::Result<Self, RenderError> {
        Ok(Self { gtag: use_gtag_context() })
    }

    pub fn on_submit(&self) {
        self.gtag.event("quiz_submit", &[("lang", "ko".into())]);
    }
}
```

In `spawn` or event handler closures (non-hook context), use `consume_gtag_context()`:

```rust
rsx! {
    button {
        onclick: move |_| {
            consume_gtag_context()
                .event("purchase", &[("currency", "KRW".into()), ("value", 12000.into())]);
        },
        "Buy"
    }
}
```

### Typed events

Define events as structs with `#[derive(GtagEvent)]` and send them with
`send` — the event name defaults to the snake_case struct name:

```rust
use dioxus_gtag::GtagEvent;

#[derive(GtagEvent)]
#[gtag(name = "purchase")]              // optional; defaults to "purchase" anyway
pub struct Purchase {
    pub value: f64,
    pub currency: String,
    #[gtag(rename = "item_id")]         // param key override
    pub sku: String,
    #[gtag(skip)]                       // not sent
    pub internal: bool,
}

gtag.send(&Purchase {
    value: 12000.0,
    currency: "KRW".to_string(),
    sku: "SKU_1".to_string(),
    internal: true,
});
```

Field values are serialized with serde, so nested types work as long as they
implement `serde::Serialize`.

For ad-hoc nested payloads (e.g. GA4 `items` arrays), use `event_json`:

```rust
gtag.event_json("purchase", serde_json::json!({
    "currency": "KRW",
    "value": 12000,
    "items": [{ "item_id": "SKU_1", "quantity": 1 }],
}));
```

## Automatic page views (`router` feature)

Call `use_gtag_page_view()` once in the root layout, inside the `Router`:

```rust
#[component]
pub fn RootLayout() -> Element {
    use_gtag_page_view();

    rsx! { Outlet::<Route> {} }
}
```

The provider configures gtag with `send_page_view: false` by default, so the
hook's page views are not double-counted. If you do not use the hook and want
gtag's own automatic initial page view instead, opt in with
`GtagConfig::send_page_view(true)`.

## Consent and opt-out

```rust
// Consent Mode v2 update after the user accepts analytics cookies
gtag.consent(ConsentUpdate::new().analytics_storage(ConsentStatus::Granted));

// Identify the signed-in user (None clears it)
gtag.set_user_id(Some("user-123"));
gtag.set_user_property("plan", "pro".into());

// Runtime kill switch: drops all subsequent calls and sets GA's
// `ga-disable-{id}` flag. Re-enabling bootstraps gtag.js if it was
// never loaded (opt-in flows starting from `GtagConfig::enabled(false)`).
gtag.set_enabled(false);
```

## API summary

| Function | Hook? | Call site |
|----------|-------|-----------|
| `use_gtag_context_provider(id)` / `_with(config)` | Yes | App root |
| `use_gtag_context()` | Yes | Component/hook body |
| `consume_gtag_context()` | No | `spawn`, event closures |
| `provide_gtag_context(ctx)` | No | Conditional provides, tests |
| `use_gtag_page_view()` (`router` feature) | Yes | Root layout inside `Router` |
