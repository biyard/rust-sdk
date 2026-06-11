/// A typed GA4 event, usually implemented via `#[derive(GtagEvent)]`:
///
/// ```rust
/// use dioxus_gtag::GtagEvent;
///
/// #[derive(GtagEvent)]
/// #[gtag(name = "purchase")]
/// pub struct Purchase {
///     pub value: f64,
///     pub currency: String,
/// }
///
/// let event = Purchase { value: 12000.0, currency: "KRW".to_string() };
/// assert_eq!(event.event_name(), "purchase");
/// ```
///
/// Send it with [`UseGtagContext::send`](crate::UseGtagContext::send).
pub trait GtagEvent {
    /// The GA4 event name, e.g. `"purchase"`.
    fn event_name(&self) -> &str;

    /// The event params as a JSON object.
    fn params(&self) -> serde_json::Value;
}
