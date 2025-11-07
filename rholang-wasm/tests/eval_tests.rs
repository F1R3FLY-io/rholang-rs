use rholang_wasm::eval_pure;

#[test]
fn eval_returns_input_verbatim() {
    let input = "new x in { x!(42) }";
    let out = eval_pure(input);
    assert_eq!(out, input);
}

#[test]
fn eval_handles_multiline_input() {
    let input = "new stdout(`rho:io:stdout`) in {\n  stdout!(\"Hello\")\n}";
    let out = eval_pure(input);
    assert_eq!(out, input);
}
