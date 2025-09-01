use rholang_vm::{api::Opcode, api::Instruction, api::Value, VM};

#[test]
fn test_addition() {
    let mut vm = VM::new();
    let program = vec![
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::ADD),
        Instruction::nullary(Opcode::HALT),
    ];
    let result = vm.execute(&program).expect("execute ok");
    assert_eq!(result, Value::Int(5));
}
