use rholang_vm::{api::Instruction, api::Opcode, api::Value, VM};

#[test]
fn test_create_list_and_concat() {
    let mut vm = VM::new();
    // Build [1,2]
    let prog = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::CREATE_LIST, 2),
        // Build [3]
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        // Concatenate => [1,2,3]
        Instruction::nullary(Opcode::CONCAT),
        Instruction::nullary(Opcode::HALT),
    ];
    let out = vm.execute(&prog).expect("exec ok");
    assert_eq!(out, Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
}

#[test]
fn test_create_tuple_and_map() {
    let mut vm = VM::new();
    // Tuple (1,2,3)
    let prog1 = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::unary(Opcode::CREATE_TUPLE, 3),
        Instruction::nullary(Opcode::HALT),
    ];
    let out1 = vm.execute(&prog1).expect("exec ok");
    assert_eq!(out1, Value::Tuple(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));

    // Map {(1 -> 2), (3 -> 4)}; push key then value per VM's CREATE_MAP pop order
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::unary(Opcode::PUSH_INT, 4),
        Instruction::unary(Opcode::CREATE_MAP, 2),
        Instruction::nullary(Opcode::HALT),
    ];
    let out2 = vm.execute(&prog2).expect("exec ok");
    assert_eq!(out2, Value::Map(vec![
        (Value::Int(1), Value::Int(2)),
        (Value::Int(3), Value::Int(4)),
    ]));
}
