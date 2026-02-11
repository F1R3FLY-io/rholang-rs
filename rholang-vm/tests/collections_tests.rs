use rholang_process::Process;
use rholang_vm::api::{Instruction, Opcode, Value};

#[test]
fn test_create_list_and_concat() {
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
    let mut process = Process::new(prog, "collections");
    let out = process.execute().expect("exec ok");
    assert_eq!(
        out,
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_create_tuple_and_map() {
    // Tuple (1,2,3)
    let prog1 = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::unary(Opcode::CREATE_TUPLE, 3),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut process1 = Process::new(prog1, "collections");
    let out1 = process1.execute().expect("exec ok");
    assert_eq!(
        out1,
        Value::Tuple(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );

    // Map {(1 -> 2), (3 -> 4)}; push key then value per VM's CREATE_MAP pop order
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::unary(Opcode::PUSH_INT, 4),
        Instruction::unary(Opcode::CREATE_MAP, 2),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut process2 = Process::new(prog2, "collections");
    let out2 = process2.execute().expect("exec ok");
    assert_eq!(
        out2,
        Value::Map(vec![
            (Value::Int(1), Value::Int(2)),
            (Value::Int(3), Value::Int(4)),
        ])
    );
}
