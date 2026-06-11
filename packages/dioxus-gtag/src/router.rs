use dioxus::prelude::*;

use crate::context::use_gtag_context;

/// Hook — sends a `page_view` on every route change (including the initial
/// route). Call it once from the root layout, inside the `Router`:
///
/// ```rust,ignore
/// #[component]
/// pub fn RootLayout() -> Element {
///     use_gtag_page_view();
///     rsx! { Outlet::<Route> {} }
/// }
/// ```
///
/// Requires the `router` feature. The provider's gtag config keeps
/// `send_page_view: false` by default, so this does not double-count.
pub fn use_gtag_page_view() {
    let ctx = use_gtag_context();
    use_effect(move || {
        // full_route_string subscribes this effect to route changes.
        let route = dioxus::router::router().full_route_string();
        ctx.page_view(&route, None);
    });
}
