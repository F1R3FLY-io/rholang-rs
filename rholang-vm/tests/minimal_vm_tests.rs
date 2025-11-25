use rholang_vm::{api::Instruction, api::Opcode, api::Process, api::Value, VM};

#[test]
fn test_addition() {
    let mut vm = VM::new();
    let program = vec![
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::ADD),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut process = Process::new(program, "minimal");
    let result = vm.execute(&mut process).expect("execute ok");
    assert_eq!(result, Value::Int(5));
}
