/// A parameter value accepted by gtag event/config calls.
///
/// Use the `From` impls to build values inline:
///
/// ```rust
/// use dioxus_gtag::GtagValue;
///
/// let params: &[(&str, GtagValue)] = &[
///     ("currency", "KRW".into()),
///     ("value", 12000.into()),
///     ("logged_in", true.into()),
/// ];
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum GtagValue {
    String(String),
    Int(i64),
    Number(f64),
    Bool(bool),
}

impl GtagValue {
    pub(crate) fn to_json(&self) -> serde_json::Value {
        match self {
            GtagValue::String(s) => serde_json::Value::String(s.clone()),
            GtagValue::Int(i) => serde_json::Value::from(*i),
            GtagValue::Number(n) => serde_json::Value::from(*n),
            GtagValue::Bool(b) => serde_json::Value::Bool(*b),
        }
    }
}

impl From<&str> for GtagValue {
    fn from(v: &str) -> Self {
        GtagValue::String(v.to_string())
    }
}

impl From<String> for GtagValue {
    fn from(v: String) -> Self {
        GtagValue::String(v)
    }
}

impl From<i32> for GtagValue {
    fn from(v: i32) -> Self {
        GtagValue::Int(v as i64)
    }
}

impl From<i64> for GtagValue {
    fn from(v: i64) -> Self {
        GtagValue::Int(v)
    }
}

impl From<u32> for GtagValue {
    fn from(v: u32) -> Self {
        GtagValue::Int(v as i64)
    }
}

impl From<f64> for GtagValue {
    fn from(v: f64) -> Self {
        GtagValue::Number(v)
    }
}

impl From<bool> for GtagValue {
    fn from(v: bool) -> Self {
        GtagValue::Bool(v)
    }
}

pub(crate) fn params_to_json(params: &[(&str, GtagValue)]) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (key, value) in params {
        map.insert(key.to_string(), value.to_json());
    }
    serde_json::Value::Object(map)
}
