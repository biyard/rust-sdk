use dioxus::prelude::*;

#[component]
pub fn RoundBubble(
    #[props(default = "24".to_string())] width: String,
    #[props(default = "24".to_string())] height: String,
    #[props(default = "#FFFFFF".to_string())] fill: String,
    #[props(default = "".to_string())] class: String,
) -> Element {
    rsx! {

        svg {
            class,
            fill,
            height,
            view_box: "0 0 24 24",
            width,
            xmlns: "http://www.w3.org/2000/svg",
            path {
                clip_rule: "evenodd",
                d: "M21 12C21.0036 13.3975 20.6771 14.7761 20.047 16.0235C18.5233 19.0722 15.4082 20.9987 12 21C10.6025 21.0036 9.2239 20.6771 7.97648 20.047L3.00003 21L3.95297 16.0235C3.3229 14.7761 2.99639 13.3975 3.00003 12C3.00135 8.59176 4.92779 5.47665 7.97648 3.95297C9.2239 3.3229 10.6025 2.99639 12 3.00003H12.5294C17.0991 3.25213 20.7479 6.90093 21 11.4706V12Z",
                fill_rule: "evenodd",
                stroke: "black",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                stroke_width: "2",
            }
            line {
                stroke: "black",
                stroke_linecap: "round",
                stroke_width: "2",
                x1: "8.5",
                x2: "15.5",
                y1: "8.96582",
                y2: "8.96582",
            }
            path {
                d: "M8.25 12.4707L13.25 12.4707",
                stroke: "black",
                stroke_linecap: "round",
                stroke_width: "2",
            }
            path {
                d: "M8.25 15.7842H10.25",
                stroke: "black",
                stroke_linecap: "round",
                stroke_width: "2",
            }
        }
    }
}

#[component]
pub fn RoundBubble2(
    #[props(default = "24".to_string())] width: String,
    #[props(default = "24".to_string())] height: String,
    #[props(default = "#FFFFFF".to_string())] fill: String,
    #[props(default = "".to_string())] class: String,
) -> Element {
    rsx! {
        svg {
            class,
            fill,
            height,
            view_box: "0 0 21 20",
            width,
            xmlns: "http://www.w3.org/2000/svg",
            path {
                clip_rule: "evenodd",
                d: "M2.83303 10C2.83 11.1646 3.10209 12.3134 3.62715 13.3529C4.89688 15.8935 7.49281 17.4989 10.333 17.5C11.4976 17.503 12.6464 17.2309 13.6859 16.7059L17.833 17.5L17.0389 13.3529C17.5639 12.3134 17.836 11.1646 17.833 10C17.8319 7.1598 16.2265 4.56388 13.6859 3.29414C12.6464 2.76908 11.4976 2.49699 10.333 2.50003H9.89183C6.08378 2.71011 3.04312 5.75077 2.83303 9.55883V10Z",
                fill_rule: "evenodd",
                stroke: "#737373",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                stroke_width: "1.5",
            }
            line {
                stroke: "#737373",
                stroke_linecap: "round",
                stroke_width: "1.5",
                transform: "matrix(-1 0 0 1 14.083 8.30469)",
                x1: "0.75",
                x2: "6.75",
                y1: "-0.75",
                y2: "-0.75",
            }
            path {
                d: "M13.458 10.3921L9.29134 10.3921",
                stroke: "#737373",
                stroke_linecap: "round",
                stroke_width: "1.5",
            }
            path {
                d: "M13.458 13.1533H11.7913",
                stroke: "#737373",
                stroke_linecap: "round",
                stroke_width: "1.5",
            }
        }
    }
}

#[component]
pub fn SquareChat(
    #[props(default = "".to_string())] class: String,
    #[props(default = "24".to_string())] width: String,
    #[props(default = "24".to_string())] height: String,
    #[props(default = "#FFFFFF".to_string())] fill: String,
) -> Element {
    rsx! {
        svg {
            class,
            width,
            height,
            view_box: "0 0 25 25",
            fill,
            xmlns: "http://www.w3.org/2000/svg",
            g { id: "Help Question",
                path {
                    id: "Ellipse 29",
                    d: "M3.18555 7.05859C3.18555 4.84945 4.97641 3.05859 7.18555 3.05859H17.1855C19.3947 3.05859 21.1855 4.84945 21.1855 7.05859V15.0586C21.1855 17.2677 19.3947 19.0586 17.1855 19.0586H14.4355L12.9855 20.9919C12.5855 21.5253 11.7855 21.5253 11.3855 20.9919L9.93555 19.0586H7.18555C4.97641 19.0586 3.18555 17.2677 3.18555 15.0586V7.05859Z",
                    stroke: "#555462",
                    stroke_width: "1.5",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                }
                line {
                    id: "Line 4",
                    x1: "7.93555",
                    y1: "6.93359",
                    x2: "16.4355",
                    y2: "6.93359",
                    stroke: "#555462",
                    stroke_width: "1.5",
                    stroke_linecap: "round",
                }
                path {
                    id: "Line 5",
                    d: "M7.68555 11.1211L13.5918 11.1211",
                    stroke: "#555462",
                    stroke_width: "1.5",
                    stroke_linecap: "round",
                }
                path {
                    id: "Line 6",
                    d: "M7.68555 14.8086H10.6855",
                    stroke: "#555462",
                    stroke_width: "1.5",
                    stroke_linecap: "round",
                }
            }
        }
    }
}

#[component]
pub fn SquareMark(#[props(default = "#FFFFFF".to_string())] color: String) -> Element {
    rsx! {
        svg {
            width: "24",
            height: "24",
            view_box: "0 0 25 24",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            g {
                path {
                    d: "M3.95752 6.82422C3.95752 4.61508 5.74838 2.82422 7.95752 2.82422H17.9575C20.1667 2.82422 21.9575 4.61508 21.9575 6.82422V14.8242C21.9575 17.0334 20.1667 18.8242 17.9575 18.8242H15.2075L13.7575 20.7576C13.3575 21.2909 12.5575 21.2909 12.1575 20.7576L10.7075 18.8242H7.95752C5.74838 18.8242 3.95752 17.0334 3.95752 14.8242V6.82422Z",
                    stroke: "{color}",
                    "stroke-width": "1.5",
                    "stroke-linecap": "round",
                    "stroke-linejoin": "round",
                }
                path {
                    d: "M17.3082 14.263V11.2908C17.3082 10.1862 16.4128 9.2908 15.3082 9.2908H8.60681M8.60681 9.2908L11.0929 6.80469M8.60681 9.2908L11.0929 11.7769",
                    stroke: "{color}",
                    "stroke-width": "1.5",
                    "stroke-linecap": "round",
                    "stroke-linejoin": "round",
                }
            }
        }
    }
}
