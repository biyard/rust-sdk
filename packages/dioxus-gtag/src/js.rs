//! Pure builders for the JavaScript snippets sent to `document::eval`.
//!
//! Kept free of Dioxus types so they can be unit-tested without a runtime.
//! All dynamic strings are embedded via `serde_json` serialization, which
//! yields properly quoted and escaped JS string literals.

use crate::config::GtagConfig;
use crate::consent::ConsentUpdate;

fn js_string(s: &str) -> String {
    serde_json::to_string(s).expect("string serialization cannot fail")
}

/// The bootstrap snippet: defines `dataLayer`/`gtag`, declares default
/// consent, configures the measurement id, and injects the gtag.js script
/// tag exactly once.
pub(crate) fn bootstrap(config: &GtagConfig) -> String {
    let id = js_string(&config.measurement_id);

    let consent = match &config.default_consent {
        Some(update) => format!("gtag('consent', 'default', {});\n", update.to_json()),
        None => String::new(),
    };

    let mut params = serde_json::Map::new();
    params.insert(
        "send_page_view".to_string(),
        serde_json::Value::Bool(config.send_page_view),
    );
    if config.debug_mode {
        params.insert("debug_mode".to_string(), serde_json::Value::Bool(true));
    }
    let params = serde_json::Value::Object(params);

    format!(
        r#"window.dataLayer = window.dataLayer || [];
window.gtag = window.gtag || function() {{ window.dataLayer.push(arguments); }};
gtag('js', new Date());
{consent}gtag('config', {id}, {params});
if (!document.getElementById('dioxus-gtag-js')) {{
    var s = document.createElement('script');
    s.id = 'dioxus-gtag-js';
    s.async = true;
    s.src = 'https://www.googletagmanager.com/gtag/js?id=' + encodeURIComponent({id});
    document.head.appendChild(s);
}}"#
    )
}

pub(crate) fn event(name: &str, params: &serde_json::Value) -> String {
    format!("gtag('event', {}, {});", js_string(name), params)
}

pub(crate) fn page_view(path: &str, title: Option<&str>) -> String {
    let mut params = serde_json::Map::new();
    params.insert(
        "page_path".to_string(),
        serde_json::Value::String(path.to_string()),
    );
    if let Some(title) = title {
        params.insert(
            "page_title".to_string(),
            serde_json::Value::String(title.to_string()),
        );
    }
    event("page_view", &serde_json::Value::Object(params))
}

pub(crate) fn set_user_id(id: Option<&str>) -> String {
    let value = match id {
        Some(id) => js_string(id),
        None => "null".to_string(),
    };
    format!("gtag('set', {{ 'user_id': {value} }});")
}

pub(crate) fn set_user_property(key: &str, value: &serde_json::Value) -> String {
    format!(
        "gtag('set', 'user_properties', {{ {}: {} }});",
        js_string(key),
        value
    )
}

pub(crate) fn consent_update(update: &ConsentUpdate) -> String {
    format!("gtag('consent', 'update', {});", update.to_json())
}

/// Toggles Google Analytics' library-level kill switch for a measurement id.
pub(crate) fn set_disabled(measurement_id: &str, disabled: bool) -> String {
    format!(
        "window['ga-disable-' + {}] = {disabled};",
        js_string(measurement_id)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consent::{ConsentStatus, ConsentUpdate};
    use crate::value::{params_to_json, GtagValue};

    #[test]
    fn bootstrap_contains_config_and_script() {
        let config = GtagConfig::new("G-TEST123").debug(true);
        let js = bootstrap(&config);
        assert!(js.contains(r#"gtag('config', "G-TEST123", {"debug_mode":true,"send_page_view":false});"#));
        assert!(js.contains("googletagmanager.com/gtag/js"));
        assert!(!js.contains("gtag('consent', 'default'"));
    }

    #[test]
    fn bootstrap_declares_default_consent_before_config() {
        let config =
            GtagConfig::new("G-TEST123").default_consent(ConsentUpdate::deny_all());
        let js = bootstrap(&config);
        let consent_pos = js.find("gtag('consent', 'default'").unwrap();
        let config_pos = js.find("gtag('config'").unwrap();
        assert!(consent_pos < config_pos);
    }

    #[test]
    fn event_escapes_name_and_params() {
        let params = params_to_json(&[
            ("currency", "KRW".into()),
            ("value", 12000.into()),
            ("note", GtagValue::from("has \"quotes\"")),
        ]);
        let js = event("purchase", &params);
        assert_eq!(
            js,
            r#"gtag('event', "purchase", {"currency":"KRW","note":"has \"quotes\"","value":12000});"#
        );
    }

    #[test]
    fn page_view_with_title() {
        assert_eq!(
            page_view("/ko/main", Some("Main")),
            r#"gtag('event', "page_view", {"page_path":"/ko/main","page_title":"Main"});"#
        );
    }

    #[test]
    fn user_id_can_be_cleared() {
        assert_eq!(
            set_user_id(Some("user-1")),
            r#"gtag('set', { 'user_id': "user-1" });"#
        );
        assert_eq!(set_user_id(None), "gtag('set', { 'user_id': null });");
    }

    #[test]
    fn consent_update_only_includes_set_fields() {
        let update = ConsentUpdate::new().analytics_storage(ConsentStatus::Granted);
        assert_eq!(
            consent_update(&update),
            r#"gtag('consent', 'update', {"analytics_storage":"granted"});"#
        );
    }

    #[test]
    fn disable_flag_uses_measurement_id() {
        assert_eq!(
            set_disabled("G-TEST123", true),
            r#"window['ga-disable-' + "G-TEST123"] = true;"#
        );
    }
}
