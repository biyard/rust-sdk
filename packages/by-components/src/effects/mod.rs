#![allow(non_snake_case)]
use dioxus::prelude::*;

/// A component that creates a shadow effect on hover.
///
/// # Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use by_components::effects::HoverEffects;
///
/// #[component]
/// pub fn App() -> Element {
///     rsx! {
///         div {
///             HoverEffects {}
///             button { "Hover me" }
///             a { "Hover me" }
///             div { class: "hover-effect", "hover me" }
///             div { "Don't hover me" }
///         }
///     }
/// }
/// ```
///
/// # Output
///
/// It shows a shadow effect on all of <button/> and <a/> tags.
/// It also shows a shadow effect on the element starting with the class name "hover-effect".
#[component]
pub fn HoverEffects() -> Element {
    #[cfg(target_arch = "wasm32")]
    {
        use_effect(move || {
            tracing::debug!("shadow mounted");
            use wasm_bindgen::prelude::*;
            use web_sys::*;

            let document = web_sys::window().unwrap().document().unwrap();
            let shadow = document.get_element_by_id("shadow").unwrap();
            let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                if let Ok(event) = event.dyn_into::<web_sys::MouseEvent>() {
                    if let Some(target) = event.target() {
                        if let Ok(el) = target.dyn_into::<HtmlElement>() {
                            if (el.tag_name().to_lowercase().as_str() == "button")
                                || (el.tag_name().to_lowercase().as_str() == "a")
                                || el
                                    .class_name()
                                    .split(" ")
                                    .collect::<Vec<&str>>()
                                    .contains(&"hover-effect")
                            {
                                let x = js_sys::Reflect::get(&event, &JsValue::from_str("pageX"))
                                    .unwrap()
                                    .as_f64()
                                    .unwrap_or(0.0) as i32;
                                let y = js_sys::Reflect::get(&event, &JsValue::from_str("pageY"))
                                    .unwrap()
                                    .as_f64()
                                    .unwrap_or(0.0) as i32;

                                shadow
                                .set_attribute("style", &format!("left: {}px; top: {}px; position: absolute; width: 50px; height: 50px; background: radial-gradient(circle, rgba(100, 100, 100, 0.5), rgba(0, 0, 0, 0)); border-radius: 50%; pointer-events: none; transform: translate(-50%, -50%); mix-blend-mode: screen; opacity: 0.7; filter: blur(15px); z-index: 9999; ", x, y))
                                .unwrap();
                            } else {
                                shadow.set_attribute("style", "display: none;").unwrap();
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(web_sys::Event)>);

            document
                .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())
                .unwrap();

            closure.forget();
        });
    }

    rsx! {
        div { id: "shadow" }
    }
}
