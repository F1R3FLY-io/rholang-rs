use rholang_vm::{api::Instruction, api::Opcode, api::Process, api::Value, VM};

fn label(name: &str) -> Value {
    Value::Str(name.to_string())
}

#[test]
fn test_nop_and_halt() {
    let mut vm = VM::new();
    let prog = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::nullary(Opcode::NOP),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::nullary(Opcode::HALT),
        // If HALT works, this next instruction must not run
        Instruction::unary(Opcode::PUSH_INT, 999),
    ];
    let mut p = Process::new(prog, "cf_nop_halt");
    let out = vm.execute(&mut p).expect("exec ok");
    assert_eq!(out, Value::Int(2));
}

#[test]
fn test_jump_to_label_and_not_found() {
    // OK jump
    let mut vm = VM::new();
    let prog = vec![
        // push label "L"
        Instruction::unary(Opcode::PUSH_STR, 0),
        Instruction::nullary(Opcode::JUMP),
        // this should be skipped by jump
        Instruction::unary(Opcode::PUSH_INT, 111),
        // label L target
        Instruction::unary(Opcode::PUSH_INT, 42),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p = Process::new(prog.clone(), "cf_jump_ok");
    p.names = vec![label("L")];
    // set label L to point to index 3 (PUSH_INT 42)
    p.set_labels([("L", 3)]);
    let out = vm.execute(&mut p).expect("exec ok");
    assert_eq!(out, Value::Int(42));

    // Not found label should error
    let mut vm2 = VM::new();
    // reuse program but process has names with label "M" and labels empty
    let mut p2 = Process::new(prog, "cf_jump_err");
    p2.names = vec![label("M")];
    let err = vm2
        .execute(&mut p2)
        .expect_err("should error for missing label");
    let msg = err.to_string().to_lowercase();
    assert!(msg.contains("label not found"));
}

#[test]
fn test_branch_true_and_false() {
    // BRANCH_TRUE: label under cond, cond on top
    // Case true: jump
    let mut vm = VM::new();
    let prog_true = vec![
        Instruction::unary(Opcode::PUSH_STR, 0),  // label "L"
        Instruction::unary(Opcode::PUSH_BOOL, 1), // true
        Instruction::nullary(Opcode::BRANCH_TRUE),
        // fallthrough path (should be skipped)
        Instruction::unary(Opcode::PUSH_INT, 1),
        // label L target
        Instruction::unary(Opcode::PUSH_INT, 10),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p_true = Process::new(prog_true, "cf_bt_true");
    p_true.names = vec![label("L")];
    p_true.set_labels([("L", 4)]); // target is index 4: PUSH_INT 10
    let out_true = vm.execute(&mut p_true).expect("exec ok");
    assert_eq!(out_true, Value::Int(10));

    // Case false: fall through
    let mut vm2 = VM::new();
    let prog_false = vec![
        Instruction::unary(Opcode::PUSH_STR, 0),
        Instruction::unary(Opcode::PUSH_BOOL, 0), // false
        Instruction::nullary(Opcode::BRANCH_TRUE),
        // fallthrough happens
        Instruction::unary(Opcode::PUSH_INT, 7),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p_false = Process::new(prog_false, "cf_bt_false");
    p_false.names = vec![label("L")];
    p_false.set_labels([("L", 99)]); // should be ignored
    let out_false = vm2.execute(&mut p_false).expect("exec ok");
    assert_eq!(out_false, Value::Int(7));

    // BRANCH_FALSE: true fallthrough, false jumps
    let mut vm3 = VM::new();
    let prog_bf = vec![
        Instruction::unary(Opcode::PUSH_STR, 0),
        Instruction::unary(Opcode::PUSH_BOOL, 0), // false -> should jump
        Instruction::nullary(Opcode::BRANCH_FALSE),
        // fallthrough (skip)
        Instruction::unary(Opcode::PUSH_INT, 99),
        // target
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p_bf = Process::new(prog_bf, "cf_bf");
    p_bf.names = vec![label("T")];
    p_bf.set_labels([("T", 4)]);
    let out_bf = vm3.execute(&mut p_bf).expect("exec ok");
    assert_eq!(out_bf, Value::Int(3));
}

#[test]
fn test_branch_success_true_and_false() {
    // success true: jump; stack: [.., label, true]
    let mut vm = VM::new();
    let prog = vec![
        Instruction::unary(Opcode::PUSH_STR, 0),
        Instruction::unary(Opcode::PUSH_BOOL, 1),
        Instruction::nullary(Opcode::BRANCH_SUCCESS),
        // fallthrough
        Instruction::unary(Opcode::PUSH_INT, 0),
        // target
        Instruction::unary(Opcode::PUSH_INT, 123),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p = Process::new(prog, "cf_bs_true");
    p.names = vec![label("S")];
    p.set_labels([("S", 4)]);
    let out = vm.execute(&mut p).expect("exec ok");
    assert_eq!(out, Value::Int(123));

    // success false: should pop label and fall through
    let mut vm2 = VM::new();
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_STR, 0),
        Instruction::unary(Opcode::PUSH_BOOL, 0),
        Instruction::nullary(Opcode::BRANCH_SUCCESS),
        Instruction::unary(Opcode::PUSH_INT, 77),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p2 = Process::new(prog2, "cf_bs_false");
    p2.names = vec![label("S")];
    let out2 = vm2.execute(&mut p2).expect("exec ok");
    assert_eq!(out2, Value::Int(77));
}

#[test]
fn test_branch_true_param_errors() {
    // Missing bool -> error
    let mut vm = VM::new();
    let prog = vec![
        Instruction::unary(Opcode::PUSH_STR, 0),
        Instruction::nullary(Opcode::BRANCH_TRUE),
    ];
    let mut p = Process::new(prog, "cf_bt_err1");
    p.names = vec![label("L")];
    let err = vm
        .execute(&mut p)
        .expect_err("should error for missing condition");
    let msg = err.to_string().to_lowercase();
    assert!(msg.contains("branch_true") && msg.contains("expects bool"));

    // Wrong label type -> error
    let mut vm2 = VM::new();
    let prog2 = vec![
        // Put non-label under condition
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::PUSH_BOOL, 1),
        Instruction::nullary(Opcode::BRANCH_TRUE),
    ];
    let mut p2 = Process::new(prog2, "cf_bt_err2");
    let err2 = vm2
        .execute(&mut p2)
        .expect_err("should error for wrong label type");
    let msg2 = err2.to_string().to_lowercase();
    assert!(msg2.contains("branch_true") && msg2.contains("label string"));
}
