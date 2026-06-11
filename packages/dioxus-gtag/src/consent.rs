/// Consent state for a single consent type, as defined by Google Consent Mode v2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentStatus {
    Granted,
    Denied,
}

impl ConsentStatus {
    fn as_str(&self) -> &'static str {
        match self {
            ConsentStatus::Granted => "granted",
            ConsentStatus::Denied => "denied",
        }
    }
}

/// A consent update for Google Consent Mode v2.
///
/// Only the fields set to `Some` are included in the `gtag('consent', …)` call,
/// so an update can change one consent type without touching the others.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConsentUpdate {
    pub ad_storage: Option<ConsentStatus>,
    pub analytics_storage: Option<ConsentStatus>,
    pub ad_user_data: Option<ConsentStatus>,
    pub ad_personalization: Option<ConsentStatus>,
}

impl ConsentUpdate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn grant_all() -> Self {
        Self {
            ad_storage: Some(ConsentStatus::Granted),
            analytics_storage: Some(ConsentStatus::Granted),
            ad_user_data: Some(ConsentStatus::Granted),
            ad_personalization: Some(ConsentStatus::Granted),
        }
    }

    pub fn deny_all() -> Self {
        Self {
            ad_storage: Some(ConsentStatus::Denied),
            analytics_storage: Some(ConsentStatus::Denied),
            ad_user_data: Some(ConsentStatus::Denied),
            ad_personalization: Some(ConsentStatus::Denied),
        }
    }

    pub fn ad_storage(mut self, status: ConsentStatus) -> Self {
        self.ad_storage = Some(status);
        self
    }

    pub fn analytics_storage(mut self, status: ConsentStatus) -> Self {
        self.analytics_storage = Some(status);
        self
    }

    pub fn ad_user_data(mut self, status: ConsentStatus) -> Self {
        self.ad_user_data = Some(status);
        self
    }

    pub fn ad_personalization(mut self, status: ConsentStatus) -> Self {
        self.ad_personalization = Some(status);
        self
    }

    pub(crate) fn to_json(self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        let fields = [
            ("ad_storage", self.ad_storage),
            ("analytics_storage", self.analytics_storage),
            ("ad_user_data", self.ad_user_data),
            ("ad_personalization", self.ad_personalization),
        ];
        for (key, status) in fields {
            if let Some(status) = status {
                map.insert(
                    key.to_string(),
                    serde_json::Value::String(status.as_str().to_string()),
                );
            }
        }
        serde_json::Value::Object(map)
    }
}
