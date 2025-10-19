#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;
use wasm_bindgen_futures::JsFuture;
use rholang_wasm_draft::WasmInterpreter;

// Do not force browser runner; allow running under Node when available.

#[wasm_bindgen_test(async)]
async fn interpret_echoes_input_for_valid_code() {
    let interp = WasmInterpreter::new();
    let code = "new x in { x!(42) }";
    let js_val = JsFuture::from(interp.interpret(code.to_string()))
        .await
        .expect("promise should resolve");
    let out = js_val.as_string().expect("expected string output");
    assert_eq!(out, code, "on wasm32 the provider echoes input");
}

#[wasm_bindgen_test(async)]
async fn interpret_returns_string_for_invalid_code() {
    let interp = WasmInterpreter::new();
    let code = "this is not valid rholang";
    let js_val = JsFuture::from(interp.interpret(code.to_string()))
        .await
        .expect("promise should resolve even for invalid input");
    let out = js_val.as_string().expect("expected string output");
    assert_eq!(out, code, "on wasm32 the provider echoes input, even if invalid");
}
