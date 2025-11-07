use pretty_assertions::assert_eq;

// Bring functions into scope from the crate under test
use rholang_wasm::{eval, eval_pure};

#[test]
fn eval_pure_echoes_input_simple() {
    let input = "1 + 2 * 3";
    let out = eval_pure(input);
    assert_eq!(out, input);
}

#[test]
fn eval_pure_handles_multiline() {
    let input = "new x in {\n  x!(42)\n}";
    let out = eval_pure(input);
    assert_eq!(out, input);
}

#[test]
fn eval_exposed_function_is_callable_natively() {
    // Although primarily for WASM, the exported function is also callable natively.
    let input = "new stdout(`rho:io:stdout`) in { stdout!(\"Hi\") }";
    let out = eval(input);
    assert_eq!(out, input);
}
