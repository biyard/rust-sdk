use dioxus::prelude::*;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::js_sys::eval;

#[component]
pub fn RichText(content: String, onchange: EventHandler<String>) -> Element {
    use_effect(move || {
        let document = web_sys::window().unwrap().document().unwrap();

        let script = r#"
                setTimeout(() => {
                    let editor = document.querySelector(".rich-text-editor");
                    if (editor && !editor.__quill) {
                        editor.__quill = new Quill(editor, { theme: "snow" });
                        editor.__quill.root.innerHTML = editor.getAttribute("data-default-value") || "";

                        let toolbar = document.querySelector(".ql-toolbar");
                        if (toolbar) {
                            toolbar.style.width = "100%";   
                            toolbar.style.maxWidth = "none"; 
                        }
                    
                        editor.__quill.on('text-change', function(delta, oldDelta, source) {
                            let new_content = editor.__quill.root.innerHTML;
                            window.__quill_content = new_content;
                            document.dispatchEvent(new Event('content-updated')); 
                        });
                    }
                }, 500);
            "#;

        spawn(async move {
            let _ = eval(&script);
        });

        let closure = Closure::wrap(Box::new(move |_event: JsValue| {
            let editor = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .query_selector(".rich-text-editor")
                .unwrap()
                .unwrap();

            let html = editor
                .dyn_ref::<web_sys::HtmlElement>()
                .unwrap()
                .inner_html();

            onchange.call(html.clone());
        }) as Box<dyn FnMut(JsValue)>);

        document
            .add_event_listener_with_callback("content-updated", closure.as_ref().unchecked_ref())
            .unwrap();

        closure.forget();
    });

    rsx! {
        div {
            class: "rich-text-editor w-full h-fit min-h-[100px] overflow-y-auto border-1 border-gray-300 rounded-md",
            "data-default-value": "{content}",
        }
    }
}
