// Simple Arithmetic Example
// Demonstrates the Rholang VM executing a basic arithmetic operation

use anyhow::Result;
use rholang_vm::bytecode::{Instruction, Label};
use rholang_vm::vm::VM;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Rholang VM Simple Arithmetic Example");
    println!("------------------------------------");

    // Create a new VM instance
    let vm = VM::new()?;
    println!("VM created successfully");

    // Create a simple bytecode program that adds two numbers
    let program = vec![
        Instruction::PushInt(2),
        Instruction::PushInt(3),
        Instruction::Add,
    ];
    println!("Bytecode program created: {:?}", program);

    // Execute the program
    println!("Executing program...");
    let result = vm.execute(&program).await?;
    println!("Result: {}", result);

    // Create a more complex program with control flow
    println!("\nExecuting a more complex program with control flow...");
    let complex_program = vec![
        Instruction::PushInt(10),
        Instruction::PushInt(5),
        Instruction::PushBool(true),
        Instruction::BranchTrue(Label("true_branch".to_string())),
        // False branch (should be skipped)
        Instruction::Add,
        Instruction::Jump(Label("end".to_string())),
        // True branch
        Instruction::Label(Label("true_branch".to_string())),
        Instruction::Sub,
        Instruction::Label(Label("end".to_string())),
    ];
    println!("Complex bytecode program created");

    // Execute the complex program
    let complex_result = vm.execute(&complex_program).await?;
    println!("Result of complex program: {}", complex_result);

    // Create a program with local variables
    println!("\nExecuting a program with local variables...");
    let locals_program = vec![
        Instruction::AllocLocal,       // Allocate local 0
        Instruction::PushInt(42),      // Push 42
        Instruction::StoreLocal(0),    // Store 42 in local 0
        Instruction::AllocLocal,       // Allocate local 1
        Instruction::PushInt(7),       // Push 7
        Instruction::StoreLocal(1),    // Store 7 in local 1
        Instruction::LoadLocal(0),     // Load local 0 (42)
        Instruction::LoadLocal(1),     // Load local 1 (7)
        Instruction::Mul,              // Multiply them
    ];
    println!("Local variables program created");

    // Execute the locals program
    let locals_result = vm.execute(&locals_program).await?;
    println!("Result of local variables program: {}", locals_result);

    Ok(())
}