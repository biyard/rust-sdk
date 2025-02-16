use js_sys::Function;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = d3)]
    pub fn create(selector: &str) -> D3Selection;

    #[wasm_bindgen(js_namespace = d3)]
    pub fn select(selector: &str) -> D3Selection;

    #[wasm_bindgen(js_namespace = d3, js_name = select)]
    pub fn select_svg(selector: &str) -> D3Selection;

    #[wasm_bindgen(js_namespace = d3, js_name = scaleOrdinal)]
    pub fn scale_ordinal() -> D3Scale;

    #[wasm_bindgen(js_namespace = d3, js_name = scaleBand)]
    pub fn scale_band() -> D3Scale;

    #[wasm_bindgen(js_namespace = d3, js_name = scaleLinear)]
    pub fn scale_linear() -> D3Scale;

    #[wasm_bindgen(js_namespace = d3)]
    pub fn pie() -> D3Pie;

    #[wasm_bindgen(js_namespace = d3)]
    pub fn arc() -> D3Arc;

    #[wasm_bindgen(js_namespace = d3, js_name = quantize)]
    pub fn quantize(f: &Function, count: usize) -> JsValue;

    #[wasm_bindgen(js_namespace = d3, js_name = interpolateSpectral)]
    pub fn interpolate_spectral(t: f64) -> JsValue;

    #[wasm_bindgen(js_namespace = d3, js_name = interpolateRainbow)]
    pub fn interpolate_rainbow(t: f64) -> JsValue;

    #[wasm_bindgen(js_namespace = d3, js_name = interpolateRgb)]
    pub fn interpolate_rgb(t: f64) -> D3InterpolateRgb;

    #[wasm_bindgen(js_namespace = d3, js_name = interpolate)]
    pub fn interpolate(t: f64, t1: f64) -> Function;

    #[wasm_bindgen(js_namespace = d3, js_name = interpolateRgbBasis)]
    pub fn interpolate_rgb_basis(t: Vec<String>) -> Function;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Object")]
    pub type D3InterpolateRgb;

    #[wasm_bindgen(method)]
    pub fn gamma(this: &D3InterpolateRgb, t: f64) -> Function;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Object")]
    pub type D3Selection;

    #[wasm_bindgen(method, js_name = append)]
    pub fn append(this: &D3Selection, element: &str) -> D3Selection;

    #[wasm_bindgen(method, js_name = attr)]
    pub fn attr(this: &D3Selection, name: &str, value: &JsValue) -> D3Selection;

    #[wasm_bindgen(method, js_name = attrTween)]
    pub fn attr_tween(this: &D3Selection, name: &str, value: &JsValue) -> D3Selection;

    #[wasm_bindgen(method, js_name = attr)]
    pub fn attr_with_str(this: &D3Selection, name: &str, value: &str) -> D3Selection;

    // attr function with an integer value
    #[wasm_bindgen(method, js_name = attr)]
    pub fn attr_with_i32(this: &D3Selection, name: &str, value: i32) -> D3Selection;

    // attr function with a floating-point number
    #[wasm_bindgen(method, js_name = attr)]
    pub fn attr_with_f64(this: &D3Selection, name: &str, value: f64) -> D3Selection;

    // attr function with a JavaScript function (closure)
    #[wasm_bindgen(method, js_name = attr)]
    pub fn attr_with_closure(this: &D3Selection, name: &str, value: &Function) -> D3Selection;

    #[wasm_bindgen(method, js_class = "Selection", js_name = text)]
    pub fn text_with_closure(this: &D3Selection, value: &Function) -> D3Selection;

    #[wasm_bindgen(method, js_name = style)]
    pub fn style(this: &D3Selection, name: &str, value: &str) -> D3Selection;

    #[wasm_bindgen(method, js_name = data)]
    pub fn data(this: &D3Selection, data: &JsValue) -> D3Selection;

    #[wasm_bindgen(method, js_name = enter)]
    pub fn enter(this: &D3Selection) -> D3Selection;

    #[wasm_bindgen(method, js_name = selectAll)]
    pub fn select_all(this: &D3Selection, selector: &str) -> D3Selection;

    #[wasm_bindgen(method, js_name = selectAll)]
    pub fn select_all0(this: &D3Selection) -> D3Selection;

    #[wasm_bindgen(method, js_name = text)]
    pub fn text(this: &D3Selection, value: &Function) -> D3Selection;

    #[wasm_bindgen(method, js_name = node)]
    pub fn node(this: &D3Selection) -> JsValue;

    #[wasm_bindgen(method, js_name = transition)]
    pub fn transition(this: &D3Selection) -> D3Selection;

    #[wasm_bindgen(method, js_name = duration)]
    pub fn duration(this: &D3Selection, time: i32) -> D3Selection;

    #[wasm_bindgen(method, js_name = duration)]
    pub fn duration_with_closure(this: &D3Selection, time: &Function) -> D3Selection;

    #[wasm_bindgen(method, js_name = delay)]
    pub fn delay(this: &D3Selection, time: &Function) -> D3Selection;

    #[wasm_bindgen(method, js_name = call)]
    pub fn call(this: &D3Selection, function: &Function) -> D3Selection;

    #[wasm_bindgen(method, js_name = join)]
    pub fn join(this: &D3Selection, element: &str) -> D3Selection;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Object")]
    pub type D3Scale;

    #[wasm_bindgen(method, js_name = domain)]
    pub fn domain(this: &D3Scale, values: &JsValue) -> D3Scale;

    #[wasm_bindgen(method, js_name = range)]
    pub fn range_scale(this: &D3Scale, values: &JsValue) -> D3Scale;

    #[wasm_bindgen(method, js_name = padding)]
    pub fn padding(this: &D3Scale, value: f64) -> D3Scale;

    #[wasm_bindgen(method, js_name = bandwidth)]
    pub fn bandwidth(this: &D3Scale) -> f64;

    #[wasm_bindgen(method, js_name = range)]
    pub fn range(this: &D3Scale, values: &JsValue) -> Function;

    #[wasm_bindgen(js_namespace = d3, js_name = axisBottom)]
    pub fn axis_bottom(scale: &D3Scale) -> Function;

    #[wasm_bindgen(js_namespace = d3, js_name = axisLeft)]
    pub fn axis_left(scale: &D3Scale) -> D3Axis;

    #[wasm_bindgen]
    pub type D3Axis;

    #[wasm_bindgen(method, js_name = ticks)]
    pub fn ticks(this: &D3Axis, count: i32) -> D3Axis;

    #[wasm_bindgen(method, js_name = tickSize)]
    pub fn tick_size(this: &D3Axis, size: f64) -> D3Axis;

    #[wasm_bindgen(method, js_name = tickFormat)]
    pub fn tick_format(this: &D3Axis, format: &Function) -> Function;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Object")]
    pub type D3Pie;

    #[wasm_bindgen(method, js_name = padAngle)]
    pub fn pad_angle(this: &D3Pie, value: f64) -> D3Pie;

    #[wasm_bindgen(method, js_name = sort)]
    pub fn sort(this: &D3Pie, value: &JsValue) -> D3Pie;

    #[wasm_bindgen(method, js_name = value)]
    pub fn value(this: &D3Pie, value: &Function) -> Function;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Object")]
    pub type D3Arc;

    #[wasm_bindgen(method, js_name = innerRadius)]
    pub fn inner_radius(this: &D3Arc, radius: f64) -> D3Arc;

    #[wasm_bindgen(method, js_name = outerRadius)]
    pub fn outer_radius(this: &D3Arc, radius: f64) -> D3Arc;

    #[wasm_bindgen(method, js_name = centroid)]
    pub fn centroid(this: &D3Arc, arc: &JsValue) -> Vec<f64>;
}
