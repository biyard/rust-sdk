#![allow(non_snake_case)]

use crate::{charts::d3, theme::ChartTheme};
use dioxus::prelude::*;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};

use super::utils::closure;

#[component]
pub fn BarChart(
    width: String,
    height: String,
    data: Vec<BarChartData>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let chart_theme: ChartTheme = try_use_context().unwrap_or_default();
    let colors = chart_theme.bar_chart_colors;

    rsx! {
        div {
            width,
            height,
            onmounted: move |_el| {
                use dioxus::web::WebEventExt;
                let el = _el.as_web_event();
                let width = el.client_width();
                let height = el.client_height();
                let svg = inject_svg(width, height, data.clone(), colors.clone());
                el.append_child(&svg).unwrap();
            },
            ..attributes,
            {children}
        }
    }
}

#[derive(Props, Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BarChartData {
    label: String,
    value: i32,
}

impl BarChartData {
    pub fn new(label: String, value: i32) -> Self {
        Self { label, value }
    }
}

fn inject_svg(
    width: i32,
    height: i32,
    data: Vec<BarChartData>,
    colors: Vec<&'static str>,
) -> web_sys::Node {
    let margin_left = 140.0;
    let margin_top = 50.0;
    let width = width as f64 - margin_left - 20.0;
    let height = height as f64 - margin_top - 20.0;
    let max_value = data.iter().map(|d| d.value).max().unwrap_or(1) as f64;
    let total = data.iter().map(|d| d.value).sum::<i32>();

    let svg = d3::create("svg")
        .attr_with_f64("width", width + margin_left + 60.0)
        .attr_with_f64("height", height + margin_top + 20.0);

    let defs = svg.append("defs");
    let gradient = defs
        .append("linearGradient")
        .attr_with_str("id", "bar-gradient")
        .attr_with_str("x1", "0%")
        .attr_with_str("y1", "0%")
        .attr_with_str("x2", "0%")
        .attr_with_str("y2", "100%");
    gradient
        .append("stop")
        .attr_with_str("offset", "0%")
        .attr_with_str("stop-color", &colors[0]);
    gradient
        .append("stop")
        .attr_with_str("offset", "100%")
        .attr_with_str("stop-color", &colors[1]);

    let main_group = svg.append("g").attr_with_str(
        "transform",
        &format!("translate({}, {})", margin_left, margin_top),
    );

    let x = d3::scale_band()
        .domain(&to_value(&data.iter().map(|d| d.label.clone()).collect::<Vec<_>>()).unwrap())
        .range_scale(&to_value(&vec![0.0, width]).unwrap())
        .padding(0.1);

    let x_axis_fn = d3::axis_bottom(&x);
    let x_axis = main_group
        .append("g")
        .attr_with_str("transform", &format!("translate(0, {})", height));
    x_axis
        .call(&x_axis_fn)
        .select_all(".domain")
        .style("stroke", "#BFC8D9");

    x_axis
        .select_all("text")
        .style("font-size", "14px")
        .style("font-weight", "bold")
        .style("fill", "#000000")
        .attr_with_str("dy", "12px");

    let y = d3::scale_linear()
        .domain(&to_value(&vec![0.0, max_value]).unwrap())
        .range_scale(&to_value(&vec![height, 0.0]).unwrap());

    let handler_y_format = Closure::wrap(Box::new({
        move |value: JsValue| -> JsValue {
            let v = value.as_f64().unwrap_or(0.0);
            let formatted = format!("{:.1}% ({:.0})", v * 100.0 / (max_value as f64), v);
            JsValue::from_str(&formatted)
        }
    }) as Box<dyn FnMut(JsValue) -> JsValue>);

    let y_axis_fn = d3::axis_left(&y)
        .ticks(5)
        .tick_size(-width)
        .tick_format(handler_y_format.as_ref().unchecked_ref());
    let y_axis = main_group
        .append("g")
        .attr_with_str("transform", "translate(0, 0)")
        .attr_with_str("class", "grid");
    y_axis.call(&y_axis_fn);
    y_axis.select_all(".tick line").style("stroke", "#EBEFF5");
    y_axis.select_all(".domain").style("stroke", "#BFC8D9");
    y_axis
        .select_all("text")
        .style("font-size", "14px")
        .style("font-weight", "bold")
        .style("fill", "#000000")
        .attr_with_str("dx", "-5px");

    let y = d3::scale_linear()
        .domain(&to_value(&vec![0.0, max_value]).unwrap())
        .range(&to_value(&vec![height, 0.0]).unwrap());

    let bar_width = x.bandwidth();

    let x = d3::scale_band()
        .domain(&to_value(&data.iter().map(|d| d.label.clone()).collect::<Vec<_>>()).unwrap())
        .range(&to_value(&vec![0.0, width]).unwrap());

    let handler_x = closure({
        let x = x.clone();
        move |d: BarChartData| {
            let x_pos = x
                .call1(&JsValue::NULL, &JsValue::from_str(&d.label))
                .unwrap()
                .as_f64()
                .unwrap_or(0.0);
            x_pos + (bar_width / 2.0 - 55.0 / 2.0 + 10.0)
        }
    });

    let handler_y = closure({
        let y = y.clone();
        move |d: BarChartData| {
            y.call1(&JsValue::NULL, &JsValue::from_f64(d.value as f64))
                .unwrap()
                .as_f64()
                .unwrap_or(0.0)
        }
    });

    let handler_height = closure({
        let y = y.clone();
        move |d: BarChartData| {
            height
                - y.call1(&JsValue::NULL, &JsValue::from_f64(d.value as f64))
                    .unwrap()
                    .as_f64()
                    .unwrap_or(0.0)
        }
    });

    main_group
        .append("g")
        .select_all0()
        .data(&to_value(&data).unwrap())
        .join("rect")
        .attr_with_str("class", "grid")
        .attr_with_closure("x", handler_x.as_ref().unchecked_ref())
        .attr_with_closure("y", handler_y.as_ref().unchecked_ref())
        .attr_with_f64("width", 55.0)
        .attr_with_str("fill", "steelblue")
        .attr_with_str("rx", "10")
        .attr_with_str("ry", "10")
        .attr_with_str("fill", "url(#bar-gradient)")
        .transition()
        .duration(1000)
        .attr_with_closure("height", handler_height.as_ref().unchecked_ref());

    let handler_text_x = closure({
        let x = x.clone();
        move |d: BarChartData| {
            let x_pos = x
                .call1(&JsValue::NULL, &JsValue::from_str(&d.label))
                .unwrap()
                .as_f64()
                .unwrap_or(0.0);
            x_pos + (bar_width / 2.0) + 10.0
        }
    });

    let handler_text_y = closure({
        let y = y.clone();
        move |d: BarChartData| {
            y.call1(&JsValue::NULL, &JsValue::from_f64(d.value as f64))
                .unwrap()
                .as_f64()
                .unwrap_or(0.0)
                - 5.0
        }
    });

    let handler_text_value = closure(move |d: BarChartData| {
        JsValue::from_str(&format!(
            "{:.1}%",
            ((d.value as f64) * 100.0 / (total as f64))
        ))
    });

    main_group
        .append("g")
        .select_all0()
        .data(&to_value(&data).unwrap())
        .join("text")
        .attr_with_closure("x", handler_text_x.as_ref().unchecked_ref())
        .attr_with_closure("y", handler_text_y.as_ref().unchecked_ref())
        .attr_with_str("text-anchor", "middle")
        .attr_with_str("fill", "#000000")
        .attr_with_str("font-size", "14px")
        .text_with_closure(handler_text_value.as_ref().unchecked_ref())
        .transition()
        .duration(1000)
        .attr_with_closure("y", handler_text_y.as_ref().unchecked_ref());

    handler_x.forget();
    handler_y.forget();
    handler_height.forget();
    handler_text_x.forget();
    handler_text_y.forget();
    handler_text_value.forget();
    handler_y_format.forget();
    svg.node().into()
}
