use rholang_vm::{api::Instruction, api::Opcode, api::Value, VM};

#[test]
fn test_mul_div_mod_neg() {
    let mut vm = VM::new();
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
    let out = vm.execute(&prog).expect("exec ok");
    assert_eq!(out, Value::Int(4));

    // NEG
    let mut vm2 = VM::new();
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 9),
        Instruction::nullary(Opcode::NEG),
        Instruction::nullary(Opcode::HALT),
    ];
    let out2 = vm2.execute(&prog2).expect("exec ok");
    assert_eq!(out2, Value::Int(-9));
}

#[test]
fn test_div_mod_by_zero_errors() {
    let mut vm = VM::new();
    // div by zero
    let prog = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::PUSH_INT, 0),
        Instruction::nullary(Opcode::DIV),
    ];
    let err = vm.execute(&prog).expect_err("should error div by zero");
    assert!(err.to_string().to_lowercase().contains("division by zero"));

    // mod by zero
    let mut vm2 = VM::new();
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::PUSH_INT, 0),
        Instruction::nullary(Opcode::MOD),
    ];
    let err2 = vm2.execute(&prog2).expect_err("should error mod by zero");
    assert!(err2.to_string().to_lowercase().contains("modulo by zero"));
}
