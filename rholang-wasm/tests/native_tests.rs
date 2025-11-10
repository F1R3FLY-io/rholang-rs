use pretty_assertions::assert_eq;

// Bring functions into scope from the crate under test
use rholang_wasm::eval;

#[test]
fn eval_returns_placeholder_simple() {
    let input = "1 + 2 * 3";
    let out = eval(input);
    assert_eq!(out, "Nil");
}

#[test]
fn eval_handles_multiline_and_returns_placeholder() {
    let input = "new x in {\n  x!(42)\n}";
    let out = eval(input);
    assert_eq!(out, "Nil");
}

#[test]
fn eval_exposed_function_is_callable_natively() {
    // Although primarily for WASM, the exported function is also callable natively.
    let input = "new stdout(`rho:io:stdout`) in { stdout!(\"Hi\") }";
    let out = eval(input);
    assert_eq!(out, "Nil");
}
