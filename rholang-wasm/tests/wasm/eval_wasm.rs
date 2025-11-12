use wasm_bindgen_test::*;
use wasm_bindgen::JsValue;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn api_smoke() {
    // version() should return a non-empty semver-ish string
    let v = rholang_wasm::version();
    assert!(!v.trim().is_empty());

    // eval_async() returns a Promise that resolves to a JSON object with `pretty`
    let promise = rholang_wasm::eval_async("new x in { x!(42) }".to_string());
    let js: JsValue = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .expect("promise resolved");

    // Expect an object with `pretty` string
    let pretty = js_sys::Reflect::get(&js, &JsValue::from_str("pretty"))
        .ok()
        .and_then(|v| v.as_string());
    assert!(pretty.is_some());
}
