#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

// Ensure tests run in a browser when using `wasm-bindgen-test --browser ...`
wasm_bindgen_test_configure!(run_in_browser);

use rholang_wasm::eval;

#[wasm_bindgen_test]
fn eval_echoes_simple_input() {
    let input = "1 + 2 * 3";
    let out = eval(input);
    assert_eq!(out, input);
}

#[wasm_bindgen_test]
fn eval_echoes_multiline_input() {
    let input = "new x in {\n  x!(42)\n}";
    let out = eval(input);
    assert_eq!(out, input);
}
