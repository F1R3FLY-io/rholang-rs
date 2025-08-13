use tokio::runtime::Runtime;
use rholang_vm::{RholangVM, bytecode::Instruction};

/// Run a bytecode program and assert that it results in an error containing the provided substring
pub fn run_and_expect_err(rt: &Runtime, vm: &RholangVM, program: Vec<Instruction>, needle: &str) {
    let res = rt.block_on(async { vm.execute(&program).await });
    assert!(res.is_err(), "Expected error, got {:?}", res);
    let err = res.err().unwrap().to_string();
    assert!(
        err.contains(needle),
        "Error '{err}' does not contain expected substring '{needle}'"
    );
}
