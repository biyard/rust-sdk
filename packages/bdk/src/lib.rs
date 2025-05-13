pub mod prelude {
    pub use by_macros;
    pub use by_macros::*;
    pub use by_types;
    pub use dioxus_translate;
    pub use dioxus_translate::*;
    pub use reqwest;
    pub use rest_api;
    pub use serde;
    pub use serde_json;
    pub use serde_urlencoded;
    pub use tracing;
    pub use validator;

    #[cfg(any(
        feature = "be",
        feature = "fe",
        feature = "web",
        feature = "mobile",
        feature = "server",
        all(feature = "server", feature = "lambda")
    ))]
    pub use btracing;
    #[cfg(any(
        feature = "fe",
        feature = "web",
        feature = "mobile",
        feature = "server",
        all(feature = "server", feature = "lambda")
    ))]
    pub use by_components;
    #[cfg(any(
        feature = "fe",
        feature = "web",
        feature = "mobile",
        feature = "server",
        all(feature = "server", feature = "lambda")
    ))]
    pub use dioxus;
    #[cfg(any(
        feature = "fe",
        feature = "web",
        feature = "mobile",
        feature = "server",
        all(feature = "server", feature = "lambda")
    ))]
    pub use dioxus::prelude::*;
    #[cfg(any(
        feature = "fe",
        feature = "web",
        feature = "mobile",
        feature = "server",
        all(feature = "server", feature = "lambda")
    ))]
    pub use dioxus_aws;
    #[cfg(any(
        feature = "fe",
        feature = "web",
        feature = "mobile",
        feature = "server",
        all(feature = "server", feature = "lambda")
    ))]
    pub use dioxus_logger;
    #[cfg(any(
        feature = "fe",
        feature = "web",
        feature = "mobile",
        feature = "server",
        all(feature = "server", feature = "lambda")
    ))]
    pub use dioxus_popup;

    #[cfg(any(feature = "server", all(feature = "server", feature = "lambda")))]
    pub use dioxus::fullstack;
    #[cfg(any(feature = "server", all(feature = "server", feature = "lambda")))]
    pub use dioxus::fullstack::prelude::*;

    #[cfg(any(feature = "be", all(feature = "be", feature = "lambda")))]
    pub use aide;
    #[cfg(any(feature = "be", all(feature = "be", feature = "lambda")))]
    pub use bigdecimal;
    #[cfg(any(feature = "be", all(feature = "be", feature = "lambda")))]
    pub use by_axum;
    #[cfg(any(feature = "be", all(feature = "be", feature = "lambda")))]
    pub use schemars;
    #[cfg(any(feature = "be", all(feature = "be", feature = "lambda")))]
    pub use schemars::JsonSchema;
    #[cfg(any(
        feature = "be",
        feature = "server",
        all(feature = "be", feature = "lambda"),
        all(feature = "server", feature = "lambda")
    ))]
    pub use sqlx;
}
