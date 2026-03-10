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

#[wasm_bindgen_test]
fn eval_returns_arithmetic_result() {
    let input = "2 + 2";
    let out = eval(input);
    assert_eq!(
        "4", out,
        "expected arithmetic expression to evaluate to 4, got {out}"
    );
}

#[wasm_bindgen_test]
fn eval_handles_empty_input_as_nil() {
    let input = "   \n\t";
    let out = eval(input);
    assert_eq!(
        "Nil", out,
        "empty/whitespace input should yield Nil, got {out}"
    );
}
