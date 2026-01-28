// Simple Arithmetic Example (new rholang-vm)
// Demonstrates the new VM executing basic arithmetic using rholang-bytecode opcodes

use anyhow::Result;
use rholang_vm::{api::Instruction, api::Opcode, api::Process, api::Value, VM};

fn main() -> Result<()> {
    println!("Rholang VM Simple Arithmetic Example (new)");
    println!("-----------------------------------------");

    // Create a new VM instance
    let vm = VM::new();
    println!("VM created successfully");

    // Program: 2 + 3 -> 5
    let program = vec![
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::ADD),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut process = Process::new(program, "simple_arithmetic: add");
    process.vm = Some(vm.clone());

    println!("Executing 2 + 3 ...");
    let result = process.execute()?;
    println!("Result: {:?}", result);
    assert_eq!(result, Value::Int(5));

    // A slightly more complex arithmetic: ((10 - 5) + 4) * 2
    let program2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 10),
        Instruction::unary(Opcode::PUSH_INT, 5),
        Instruction::nullary(Opcode::SUB), // 10 - 5 = 5
        Instruction::unary(Opcode::PUSH_INT, 4),
        Instruction::nullary(Opcode::ADD), // 5 + 4 = 9
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::nullary(Opcode::MUL), // 9 * 2 = 18
        Instruction::nullary(Opcode::HALT),
    ];
    let mut process2 = Process::new(program2, "simple_arithmetic: complex");
    process2.vm = Some(vm);

    println!("Executing ((10 - 5) + 4) * 2 ...");
    let result2 = process2.execute()?;
    println!("Result: {:?}", result2);
    assert_eq!(result2, Value::Int(18));

    Ok(())
}
