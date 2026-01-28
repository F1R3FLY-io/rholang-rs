use rholang_vm::{api::Instruction, api::Opcode, api::Process, api::Value, VM};

#[test]
fn test_mul_div_mod_neg() {
    let vm = VM::new();
    // ((6 * 7) / 3) % 5 => (42/3)=14; 14%5=4
    let prog = vec![
        Instruction::unary(Opcode::PUSH_INT, 6),
        Instruction::unary(Opcode::PUSH_INT, 7),
        Instruction::nullary(Opcode::MUL),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::DIV),
        Instruction::unary(Opcode::PUSH_INT, 5),
        Instruction::nullary(Opcode::MOD),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut process = Process::new(prog, "arithmetic");
    process.vm = Some(vm);
    let out = process.execute().expect("exec ok");
    assert_eq!(out, Value::Int(4));

    // NEG
    let vm2 = VM::new();
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 9),
        Instruction::nullary(Opcode::NEG),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut process2 = Process::new(prog2, "arithmetic");
    process2.vm = Some(vm2);
    let out2 = process2.execute().expect("exec ok");
    assert_eq!(out2, Value::Int(-9));
}

#[test]
fn test_div_mod_by_zero_errors() {
    let vm = VM::new();
    // div by zero
    let prog = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::PUSH_INT, 0),
        Instruction::nullary(Opcode::DIV),
    ];
    let mut process3 = Process::new(prog, "arithmetic");
    process3.vm = Some(vm);
    let err = process3.execute().expect_err("should error div by zero");
    assert!(err.to_string().to_lowercase().contains("division by zero"));

    // mod by zero
    let vm2 = VM::new();
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::PUSH_INT, 0),
        Instruction::nullary(Opcode::MOD),
    ];
    let mut process4 = Process::new(prog2, "arithmetic");
    process4.vm = Some(vm2);
    let err2 = process4.execute().expect_err("should error mod by zero");
    assert!(err2.to_string().to_lowercase().contains("modulo by zero"));
}
