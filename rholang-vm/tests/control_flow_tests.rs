use rholang_process::Process;
use rholang_vm::api::{Instruction, Opcode, Value};

#[test]
fn test_nop_and_halt() {
    let prog = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::nullary(Opcode::NOP),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::nullary(Opcode::HALT),
        // If HALT works, this next instruction must not run
        Instruction::unary(Opcode::PUSH_INT, 999),
    ];
    let mut p = Process::new(prog, "cf_nop_halt");
    let out = p.execute().expect("exec ok");
    assert_eq!(out, Value::Int(2));
}

#[test]
fn test_jump_to_index() {
    // Jump uses an immediate absolute index now.
    let prog = vec![
        // Jump to index 3 (0-based)
        Instruction::unary(Opcode::JUMP, 3),
        // this should be skipped by jump
        Instruction::unary(Opcode::PUSH_INT, 111),
        // also skipped
        Instruction::unary(Opcode::PUSH_INT, 222),
        // target
        Instruction::unary(Opcode::PUSH_INT, 42),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p = Process::new(prog.clone(), "cf_jump_ok");
    let out = p.execute().expect("exec ok");
    assert_eq!(out, Value::Int(42));
}

#[test]
fn test_branch_true_and_false() {
    // BRANCH_TRUE immediate index
    // Case true: jump
    let prog_true = vec![
        Instruction::unary(Opcode::PUSH_BOOL, 1),   // true
        Instruction::unary(Opcode::BRANCH_TRUE, 3), // jump to index 3
        // fallthrough path (should be skipped)
        Instruction::unary(Opcode::PUSH_INT, 1),
        // target
        Instruction::unary(Opcode::PUSH_INT, 10),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p_true = Process::new(prog_true, "cf_bt_true");
    let out_true = p_true.execute().expect("exec ok");
    assert_eq!(out_true, Value::Int(10));

    // Case false: fall through
    let prog_false = vec![
        Instruction::unary(Opcode::PUSH_BOOL, 0),   // false
        Instruction::unary(Opcode::BRANCH_TRUE, 3), // would jump to index 3 if true
        // fallthrough happens
        Instruction::unary(Opcode::PUSH_INT, 7),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p_false = Process::new(prog_false, "cf_bt_false");
    let out_false = p_false.execute().expect("exec ok");
    assert_eq!(out_false, Value::Int(7));

    // BRANCH_FALSE: true fallthrough, false jumps
    let prog_bf = vec![
        Instruction::unary(Opcode::PUSH_BOOL, 0), // false -> should jump
        Instruction::unary(Opcode::BRANCH_FALSE, 3), // jump to index 3
        // fallthrough (skip)
        Instruction::unary(Opcode::PUSH_INT, 99),
        // target
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p_bf = Process::new(prog_bf, "cf_bf");
    let out_bf = p_bf.execute().expect("exec ok");
    assert_eq!(out_bf, Value::Int(3));
}

#[test]
fn test_branch_success_true_and_false() {
    // success true: jump using immediate target index
    let prog = vec![
        Instruction::unary(Opcode::PUSH_BOOL, 1),
        Instruction::unary(Opcode::BRANCH_SUCCESS, 3),
        // fallthrough
        Instruction::unary(Opcode::PUSH_INT, 0),
        // target
        Instruction::unary(Opcode::PUSH_INT, 123),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p = Process::new(prog, "cf_bs_true");
    let out = p.execute().expect("exec ok");
    assert_eq!(out, Value::Int(123));

    // success false: should fall through
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_BOOL, 0),
        Instruction::unary(Opcode::BRANCH_SUCCESS, 3),
        Instruction::unary(Opcode::PUSH_INT, 77),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p2 = Process::new(prog2, "cf_bs_false");
    let out2 = p2.execute().expect("exec ok");
    assert_eq!(out2, Value::Int(77));
}

#[test]
fn test_branch_true_param_errors() {
    // Missing bool -> error
    let prog = vec![
        Instruction::unary(Opcode::BRANCH_TRUE, 0), // target irrelevant
    ];
    let mut p = Process::new(prog, "cf_bt_err1");
    let err = p.execute().expect_err("should error for missing condition");
    let msg = err.to_string().to_lowercase();
    assert!(msg.contains("branch_true") && msg.contains("expects bool"));

    // Wrong type for condition -> error
    let prog2 = vec![
        // Put non-bool on stack
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::BRANCH_TRUE, 0),
    ];
    let mut p2 = Process::new(prog2, "cf_bt_err2");
    let err2 = p2.execute().expect_err("should error for wrong cond type");
    let msg2 = err2.to_string().to_lowercase();
    assert!(msg2.contains("branch_true") && msg2.contains("expects bool"));
}
