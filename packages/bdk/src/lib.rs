pub mod prelude {
    pub use btracing;
    pub use by_macros;
    pub use by_macros::*;
    pub use by_types;
    pub use dioxus_translate;
    pub use dioxus_translate::*;
    pub use reqwest;

    #[cfg(any(
        feature = "be",
        feature = "server",
        all(feature = "server", feature = "lambda")
    ))]
    pub use rest_api;
    pub use serde;
    pub use serde_json;
    pub use serde_urlencoded;
    pub use tracing;
    pub use validator;

    #[cfg(any(
        feature = "be",
        feature = "server",
        all(feature = "server", feature = "lambda")
    ))]
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
