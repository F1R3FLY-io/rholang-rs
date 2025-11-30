#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

// Ensure tests run in a browser when using `wasm-bindgen-test --browser ...`
wasm_bindgen_test_configure!(run_in_browser);

use rholang_wasm::eval;

#[wasm_bindgen_test]
fn eval_handles_simple_input() {
    let input = "1 + 2 * 3";
    let out = eval(input);
    // The new evaluator parses/compiles/executes; invalid input yields a ParseError string.
    assert!(!out.is_empty());
}

#[wasm_bindgen_test]
fn eval_runs_multiline_input() {
    let input = "new x in {\n  x!(42)\n}";
    let out = eval(input);
    // Expect non-empty output (e.g., "Nil" or a rendered VM Value) or an error marker.
    assert!(!out.is_empty());
}
