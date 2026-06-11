use crate::consent::ConsentUpdate;

/// Configuration for the gtag context.
///
/// ```rust
/// use dioxus_gtag::GtagConfig;
///
/// let config = GtagConfig::new("G-XXXXXXXXXX")
///     .debug(cfg!(debug_assertions))
///     .enabled(true);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GtagConfig {
    pub measurement_id: String,
    /// Whether gtag.js sends its automatic initial `page_view`.
    /// Defaults to `false` so that `use_gtag_page_view`/manual `page_view`
    /// calls do not double-count.
    pub send_page_view: bool,
    /// Attaches `debug_mode: true` so events show up in GA4 DebugView.
    pub debug_mode: bool,
    /// When `false`, gtag.js is not loaded and all calls are dropped until
    /// `set_enabled(true)` is called (e.g. after user opt-in).
    pub enabled: bool,
    /// Consent state declared before `gtag('config', …)`, per Consent Mode v2.
    pub default_consent: Option<ConsentUpdate>,
}

impl GtagConfig {
    pub fn new(measurement_id: impl Into<String>) -> Self {
        Self {
            measurement_id: measurement_id.into(),
            send_page_view: false,
            debug_mode: false,
            enabled: true,
            default_consent: None,
        }
    }

    pub fn send_page_view(mut self, on: bool) -> Self {
        self.send_page_view = on;
        self
    }

    pub fn debug(mut self, on: bool) -> Self {
        self.debug_mode = on;
        self
    }

    pub fn enabled(mut self, on: bool) -> Self {
        self.enabled = on;
        self
    }

    pub fn default_consent(mut self, consent: ConsentUpdate) -> Self {
        self.default_consent = Some(consent);
        self
    }
}
