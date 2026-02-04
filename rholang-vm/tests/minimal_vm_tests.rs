use rholang_process::Process;
use rholang_vm::api::{Instruction, Opcode, Value};

#[test]
fn test_addition() {
    let program = vec![
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::ADD),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut process = Process::new(program, "minimal");
    let result = process.execute().expect("execute ok");
    assert_eq!(result, Value::Int(5));
}
