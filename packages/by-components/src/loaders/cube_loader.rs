use dioxus::prelude::*;

#[component]
pub fn CubeLoader(
    #[props(default = "#3B3E59ff".to_string())] cube_color: String,
    #[props(default = "#ffffffff".to_string())] cube_border_color: String,
) -> Element {
    let size = "80px";
    let css = include_str!("cube_loader.css");

    rsx! {
        style { "{css}" }
        div {
            class: "scene",
            height: "calc({size} * 2.75)",
            width: "calc({size} * 2.75)",
            div { class: "cube-wrapper",
                div { class: "cube",
                    div { class: "cube-faces", width: size, height: size,
                        div {
                            class: "cube-face shadow",
                            background: "{cube_color}",
                            border: "solid 1px {cube_border_color}",
                        }
                        div {
                            class: "cube-face bottom",
                            background: "{cube_color}",
                            border: "solid 1px {cube_border_color}",
                        }
                        div {
                            class: "cube-face top",
                            background: "{cube_color}",
                            border: "solid 1px {cube_border_color}",
                        }
                        div {
                            class: "cube-face left",
                            background: "{cube_color}",
                            border: "solid 1px {cube_border_color}",
                        }
                        div {
                            class: "cube-face right",
                            background: "{cube_color}",
                            border: "solid 1px {cube_border_color}",
                        }
                        div {
                            class: "cube-face back",
                            background: "{cube_color}",
                            border: "solid 1px {cube_border_color}",
                        }
                        div {
                            class: "cube-face front",
                            background: "{cube_color}",
                            border: "solid 1px {cube_border_color}",
                        }
                    }
                }
            }
        }
    }
}
