use edgesvg::{
    analyze_bytes, compare_bytes, optimize, render_png, vectorize_bytes, VectorizeRequest,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Warn).ok();
}

#[wasm_bindgen]
pub fn vectorize(input: &[u8], request: JsValue) -> Result<JsValue, JsError> {
    let request: VectorizeRequest = if request.is_undefined() || request.is_null() {
        VectorizeRequest::default()
    } else {
        serde_wasm_bindgen::from_value(request).map_err(|err| JsError::new(&err.to_string()))?
    };
    let response =
        vectorize_bytes(input, &request).map_err(|err| JsError::new(&err.to_string()))?;
    serde_wasm_bindgen::to_value(&response).map_err(|err| JsError::new(&err.to_string()))
}

#[wasm_bindgen]
pub fn analyze(input: &[u8]) -> Result<JsValue, JsError> {
    let response = analyze_bytes(input).map_err(|err| JsError::new(&err.to_string()))?;
    serde_wasm_bindgen::to_value(&response).map_err(|err| JsError::new(&err.to_string()))
}

#[wasm_bindgen]
pub fn compare(input: &[u8], svg: &str) -> Result<JsValue, JsError> {
    let response = compare_bytes(input, svg).map_err(|err| JsError::new(&err.to_string()))?;
    serde_wasm_bindgen::to_value(&response).map_err(|err| JsError::new(&err.to_string()))
}

#[wasm_bindgen]
pub fn optimize_svg(svg: &str, precision: Option<u32>) -> Result<JsValue, JsError> {
    let response = optimize(svg, precision.unwrap_or(2));
    serde_wasm_bindgen::to_value(&response).map_err(|err| JsError::new(&err.to_string()))
}

#[wasm_bindgen]
pub fn render(svg: &str, width: u32, height: u32) -> Result<Vec<u8>, JsError> {
    render_png(svg, width, height).map_err(|err| JsError::new(&err.to_string()))
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
