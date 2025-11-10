use rholang_wasm::eval;

#[test]
fn eval_returns_vm_result_placeholder() {
    let input = "new x in { x!(42) }";
    let out = eval(input);
    // Until parsing and codegen are wired, VM runs an empty process and returns Nil
    assert_eq!(out, "Nil");
}

#[test]
fn eval_handles_multiline_input_and_returns_placeholder() {
    let input = "new stdout(`rho:io:stdout`) in {\n  stdout!(\"Hello\")\n}";
    let out = eval(input);
    assert_eq!(out, "Nil");
}
