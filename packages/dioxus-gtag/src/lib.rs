//! Google Analytics 4 (gtag.js) integration for Dioxus.
//!
//! Provide the context once at the App root, then send events from anywhere:
//!
//! ```rust,ignore
//! use dioxus::prelude::*;
//! use dioxus_gtag::*;
//!
//! #[component]
//! pub fn App() -> Element {
//!     use_gtag_context_provider("G-XXXXXXXXXX");
//!     rsx! { Router::<Route> {} }
//! }
//!
//! #[component]
//! pub fn BuyButton() -> Element {
//!     rsx! {
//!         button {
//!             onclick: move |_| {
//!                 consume_gtag_context()
//!                     .event("purchase", &[("currency", "KRW".into()), ("value", 12000.into())]);
//!             },
//!             "Buy"
//!         }
//!     }
//! }
//! ```
//!
//! The provider injects gtag.js through `document::eval`, so it works on web
//! and desktop (webview) and is a no-op during SSR — no `index.html` edits and
//! no `#[cfg]` at call sites. Events fired before gtag.js is bootstrapped are
//! buffered and flushed on initialization.
//!
//! With the `router` feature, [`use_gtag_page_view`] tracks route changes
//! automatically.

mod config;
mod consent;
mod context;
mod js;
#[cfg(feature = "router")]
mod router;
mod value;

pub use config::GtagConfig;
pub use consent::{ConsentStatus, ConsentUpdate};
pub use context::{
    consume_gtag_context, provide_gtag_context, use_gtag_context, use_gtag_context_provider,
    use_gtag_context_provider_with, UseGtagContext,
};
#[cfg(feature = "router")]
pub use router::use_gtag_page_view;
pub use value::GtagValue;

pub mod prelude {
    pub use crate::config::GtagConfig;
    pub use crate::consent::{ConsentStatus, ConsentUpdate};
    pub use crate::context::{
        consume_gtag_context, provide_gtag_context, use_gtag_context, use_gtag_context_provider,
        use_gtag_context_provider_with, UseGtagContext,
    };
    #[cfg(feature = "router")]
    pub use crate::router::use_gtag_page_view;
    pub use crate::value::GtagValue;
}
