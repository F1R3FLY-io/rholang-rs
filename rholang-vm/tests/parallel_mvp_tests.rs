#![cfg(feature = "parallel-exec")]
use std::sync::Arc;
use rholang_vm::api::{Opcode, Instruction, Value, Process};
use rholang_vm::api::VmParallel;

fn mk_add_process(a: i16, b: i16) -> Process {
    // Program: PUSH_INT a; PUSH_INT b; ADD; HALT
    let mut code = Vec::new();
    code.push(Instruction::unary(Opcode::PUSH_INT, a as u16));
    code.push(Instruction::unary(Opcode::PUSH_INT, b as u16));
    code.push(Instruction::nullary(Opcode::ADD));
    code.push(Instruction::nullary(Opcode::HALT));
    let mut p = Process::new(code, "test");
    p
}

#[test]
fn runs_many_add_processes_in_parallel_deterministically() {
    let mut vm = VmParallel::builder().threads(2).build();
    let n = 10;
    for i in 0..n { let p = mk_add_process(i, i+1); vm.spawn_process(Arc::new(p)); }
    let results = vm.run_until_quiescence();
    assert_eq!(results.len(), n as usize);
    // Because journal returns outputs ordered by seq, these must be in the same order as enqueued
    for (idx, (_pid, val)) in results.iter().enumerate() {
        let i = idx as i16;
        assert_eq!(*val, Value::Int((i + (i+1)) as i64));
    }
}
