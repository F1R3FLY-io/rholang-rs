use rholang_vm::{api::Instruction, api::Opcode, api::Process, api::Value, VM};

#[test]
fn test_addition() {
    let vm = VM::new();
    let program = vec![
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::ADD),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut process = Process::new(program, "minimal");
    process.vm = Some(vm);
    let result = process.execute().expect("execute ok");
    assert_eq!(result, Value::Int(5));
}
