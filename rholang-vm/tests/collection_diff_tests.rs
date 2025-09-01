use rholang_vm::{api::Instruction, api::Opcode, api::Value, VM};

#[test]
fn test_list_diff_basic() {
    let mut vm = VM::new();
    // [1,2,2,3] DIFF [2,4] => [1,2,3]
    let prog = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::unary(Opcode::CREATE_LIST, 4),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 4),
        Instruction::unary(Opcode::CREATE_LIST, 2),
        Instruction::nullary(Opcode::DIFF),
        Instruction::nullary(Opcode::HALT),
    ];
    let out = vm.execute(&prog).expect("exec ok");
    assert_eq!(out, Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
}

#[test]
fn test_list_diff_no_overlap() {
    let mut vm = VM::new();
    // [1,2] DIFF [3] => [1,2]
    let prog = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::CREATE_LIST, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::nullary(Opcode::DIFF),
        Instruction::nullary(Opcode::HALT),
    ];
    let out = vm.execute(&prog).expect("exec ok");
    assert_eq!(out, Value::List(vec![Value::Int(1), Value::Int(2)]));
}
