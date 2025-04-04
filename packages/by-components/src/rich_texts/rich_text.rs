use dioxus::prelude::*;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::js_sys::eval;

#[component]
pub fn RichText(
    #[props(default = "rich-text".to_string())] id: String,
    content: String,
    onchange: EventHandler<String>,
) -> Element {
    use_effect({
        let id = id.clone();
        move || {
            let id = id.clone();
            let js = format!(
                r#"
                (function tryInit() {{
                    let editor = document.getElementById("{id}");
                    if (editor && window.Quill && !editor.__quill) {{
                        editor.__quill = new Quill(editor, {{ theme: "snow" }});

                        let toolbar = editor.previousElementSibling;
                        if (toolbar) {{
                            toolbar.style.width = "100%";
                            toolbar.style.maxWidth = "none";
                            toolbar.style.boxSizing = "border-box";
                        }}

                        editor.__quill.on('text-change', function(delta, oldDelta, source) {{
                            window.__quill_content = editor.__quill.root.innerHTML;
                            document.dispatchEvent(new Event('content-updated'));
                        }});
                    }} else {{
                        setTimeout(tryInit, 200);
                    }}
                }})();
                "#
            );

            spawn(async move {
                let _ = eval(&js);
            });

            let closure = Closure::wrap(Box::new(move |_event: JsValue| {
                let editor = web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id(&id)
                    .unwrap();

                let html = editor
                    .dyn_ref::<web_sys::HtmlElement>()
                    .unwrap()
                    .inner_html();

                onchange.call(html);
            }) as Box<dyn FnMut(JsValue)>);

            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .add_event_listener_with_callback(
                    "content-updated",
                    closure.as_ref().unchecked_ref(),
                )
                .unwrap();

            closure.forget();
        }
    });

    use_effect({
        let id = id.clone();
        let content = content.clone();
        move || {
            let js = format!(
                r#"
                (function syncContent() {{
                    let editor = document.getElementById("{id}");
                    if (editor && editor.__quill) {{
                        let current = editor.__quill.root.innerHTML;
                        let next = `{content}`;
                        if (current !== next) {{
                            editor.__quill.clipboard.dangerouslyPasteHTML(next);
                        }}
                    }}
                }})();
                "#
            );

            spawn(async move {
                let _ = eval(&js);
            });
        }
    });

    rsx! {
        document::Link {
            rel: "stylesheet",
            href: "https://cdn.jsdelivr.net/npm/quill@2.0.0-dev.4/dist/quill.snow.css",
        }
        document::Script { src: "https://cdn.jsdelivr.net/npm/quill@2.0.0-dev.4/dist/quill.min.js" }

        div {
            id: "{id}",
            class: "rich-text-editor w-full h-fit min-h-[100px] overflow-y-auto border border-gray-300 rounded-md",
        }
    }
}
