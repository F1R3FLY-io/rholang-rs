use rholang_wasm::eval;

#[test]
fn eval_returns_vm_result_placeholder() {
    let input = "new x in { x!(42) }";
    let out = eval(input);
    // Sending returns Bool(true) for success
    assert_eq!(out, "true");
}

#[test]
fn eval_handles_multiline_input_and_returns_placeholder() {
    let input = "new stdout(`rho:io:stdout`) in {\n  stdout!(\"Hello\")\n}";
    let out = eval(input);
    assert_eq!(out, "true");
}
