use by_macros::DioxusController;
use dioxus::prelude::*;

use crate::config::GtagConfig;
use crate::consent::ConsentUpdate;
use crate::js;
use crate::value::{params_to_json, GtagValue};

/// App-wide Google Analytics context.
///
/// Provide it once at the App root with [`use_gtag_context_provider`], then
/// read it with [`use_gtag_context`] (components/hooks) or
/// [`consume_gtag_context`] (spawn/event closures).
#[derive(Clone, Copy, DioxusController)]
pub struct UseGtagContext {
    config: Signal<GtagConfig>,
    enabled: Signal<bool>,
    initialized: Signal<bool>,
    pending: Signal<Vec<String>>,
}

/// Hook — call once at the App root (before `Router`), like other context
/// providers. Injects gtag.js and registers the context.
///
/// ```rust,ignore
/// #[component]
/// pub fn App() -> Element {
///     use_gtag_context_provider("G-XXXXXXXXXX");
///     rsx! { Router::<Route> {} }
/// }
/// ```
pub fn use_gtag_context_provider(measurement_id: &str) -> UseGtagContext {
    use_gtag_context_provider_with(GtagConfig::new(measurement_id))
}

/// Hook — same as [`use_gtag_context_provider`] but with full [`GtagConfig`]
/// control (debug mode, default consent, opt-in flows, …).
pub fn use_gtag_context_provider_with(config: GtagConfig) -> UseGtagContext {
    let enabled = config.enabled;
    let ctx = use_context_provider(|| UseGtagContext {
        config: Signal::new(config),
        enabled: Signal::new(enabled),
        initialized: Signal::new(false),
        pending: Signal::new(Vec::new()),
    });

    // Effects only run on the client, so gtag.js is never touched during SSR.
    // Reading `enabled` subscribes the effect, so a later `set_enabled(true)`
    // (user opt-in) triggers initialization.
    use_effect(move || {
        if *ctx.enabled.read() {
            ctx.init();
        }
    });

    ctx
}

/// Hook — component or hook body.
pub fn use_gtag_context() -> UseGtagContext {
    use_context::<UseGtagContext>()
}

/// Non-hook — `spawn`, event handler closures, and other non-hook call sites.
pub fn consume_gtag_context() -> UseGtagContext {
    consume_context::<UseGtagContext>()
}

/// Non-hook — conditional provides and test setup.
pub fn provide_gtag_context(ctx: UseGtagContext) -> UseGtagContext {
    provide_context(ctx)
}

impl UseGtagContext {
    /// Sends a GA4 event: `gtag('event', name, params)`.
    ///
    /// Calls made before gtag.js is bootstrapped are buffered and flushed on
    /// initialization; calls made while disabled are dropped.
    pub fn event(&self, name: &str, params: &[(&str, GtagValue)]) {
        self.push(js::event(name, &params_to_json(params)));
    }

    /// Sends a GA4 event with arbitrary JSON params, for payloads that do not
    /// fit the `&[(&str, GtagValue)]` shape (nested items arrays, …).
    pub fn event_json(&self, name: &str, params: serde_json::Value) {
        self.push(js::event(name, &params));
    }

    /// Sends a `page_view` event. With the `router` feature,
    /// `use_gtag_page_view` calls this automatically on route changes.
    pub fn page_view(&self, path: &str, title: Option<&str>) {
        self.push(js::page_view(path, title));
    }

    /// Sets (or clears, with `None`) the GA4 `user_id` for subsequent events.
    pub fn set_user_id(&self, id: Option<&str>) {
        self.push(js::set_user_id(id));
    }

    /// Sets a GA4 user property for subsequent events.
    pub fn set_user_property(&self, key: &str, value: GtagValue) {
        self.push(js::set_user_property(key, &value.to_json()));
    }

    /// Sends a Consent Mode v2 update: `gtag('consent', 'update', …)`.
    pub fn consent(&self, update: ConsentUpdate) {
        self.push(js::consent_update(&update));
    }

    /// Runtime opt-in/opt-out toggle. Disabling sets GA's `ga-disable-{id}`
    /// kill switch and drops subsequent calls; enabling bootstraps gtag.js if
    /// it has not been loaded yet (opt-in consent flows).
    pub fn set_enabled(&self, on: bool) {
        let mut enabled = self.enabled;
        enabled.set(on);
        if *self.initialized.peek() {
            let id = self.config.peek().measurement_id.clone();
            document::eval(&js::set_disabled(&id, !on));
        }
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.peek()
    }

    fn init(&self) {
        if *self.initialized.peek() {
            // Re-enabled after an opt-out: lift the kill switch again.
            let id = self.config.peek().measurement_id.clone();
            document::eval(&js::set_disabled(&id, false));
            return;
        }

        document::eval(&js::bootstrap(&self.config.peek()));

        let mut initialized = self.initialized;
        initialized.set(true);

        let mut pending = self.pending;
        let buffered: Vec<String> = pending.write().drain(..).collect();
        for script in buffered {
            document::eval(&script);
        }
    }

    fn push(&self, script: String) {
        if !*self.enabled.peek() {
            return;
        }
        if *self.initialized.peek() {
            document::eval(&script);
        } else {
            let mut pending = self.pending;
            pending.write().push(script);
        }
    }
}
