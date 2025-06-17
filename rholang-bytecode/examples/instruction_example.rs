//! Example demonstrating the use of instructions in the rholang-bytecode crate.

use rholang_bytecode::{Constant, Instruction, Literal, Name, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Rholang Bytecode Instruction Example");
    println!("====================================");

    // Creating basic instructions
    println!("\nBasic Instructions:");
    let push_int = Instruction::push_int(42);
    let push_string = Instruction::push_string("Hello, Rholang!");
    let add = Instruction::Add;
    let pop = Instruction::Pop;

    println!("  {}", push_int);
    println!("  {}", push_string);
    println!("  {}", add);
    println!("  {}", pop);

    // Creating a simple program (calculate 2 + 3 * 4)
    println!("\nSimple Arithmetic Program:");
    let arithmetic_program = vec![
        Instruction::push_int(2),   // Push 2 onto the stack
        Instruction::push_int(3),   // Push 3 onto the stack
        Instruction::push_int(4),   // Push 4 onto the stack
        Instruction::Mul,           // Multiply 3 * 4 = 12
        Instruction::Add,           // Add 2 + 12 = 14
    ];

    for (i, instruction) in arithmetic_program.iter().enumerate() {
        println!("  {}: {}", i, instruction);
    }

    // Creating a program with control flow
    println!("\nControl Flow Program:");
    let control_flow_program = vec![
        Instruction::push_int(10),          // Push 10 onto the stack
        Instruction::push_int(20),          // Push 20 onto the stack
        Instruction::Lt,                    // Check if 10 < 20
        Instruction::JumpIfNot { target: 6 }, // Jump to instruction 6 if not true
        Instruction::push_string("10 is less than 20"), // Push result string
        Instruction::Jump { target: 7 },    // Jump to end
        Instruction::push_string("10 is not less than 20"), // Alternative result
    ];

    for (i, instruction) in control_flow_program.iter().enumerate() {
        println!("  {}: {}", i, instruction);
    }

    // Creating a program with process operations
    println!("\nProcess Operations Program:");
    let process_program = vec![
        // Create a new name
        Instruction::New { count: 1 },
        // Duplicate the name for both send and receive
        Instruction::Dup,
        
        // First process: send a message
        Instruction::push_int(42),
        Instruction::Send { arity: 1 },
        
        // Second process: receive a message
        Instruction::Receive { arity: 1, persistent: false },
        
        // Combine the processes with parallel composition
        Instruction::Par,
    ];

    for (i, instruction) in process_program.iter().enumerate() {
        println!("  {}: {}", i, instruction);
    }

    // Creating a program with data structure operations
    println!("\nData Structure Operations Program:");
    let data_structure_program = vec![
        // Create a list [1, 2, 3]
        Instruction::ListNew,
        Instruction::push_int(1),
        Instruction::ListPush,
        Instruction::push_int(2),
        Instruction::ListPush,
        Instruction::push_int(3),
        Instruction::ListPush,
        
        // Create a map {"name": "Alice", "age": 30}
        Instruction::MapNew,
        Instruction::push_string("name"),
        Instruction::push_string("Alice"),
        Instruction::MapInsert,
        Instruction::push_string("age"),
        Instruction::push_int(30),
        Instruction::MapInsert,
        
        // Create a tuple (true, "hello", 42)
        Instruction::TupleNew { size: 3 },
        Instruction::push_bool(true),
        Instruction::push_string("hello"),
        Instruction::push_int(42),
    ];

    for (i, instruction) in data_structure_program.iter().enumerate() {
        println!("  {}: {}", i, instruction);
    }

    // Serializing instructions to JSON
    println!("\nSerialization Example:");
    let instructions_to_serialize = vec![
        Instruction::push_int(42),
        Instruction::Add,
        Instruction::Jump { target: 10 },
    ];
    
    let json = serde_json::to_string_pretty(&instructions_to_serialize)?;
    println!("  JSON representation:\n{}", json);
    
    // Deserializing instructions from JSON
    let deserialized: Vec<Instruction> = serde_json::from_str(&json)?;
    println!("\n  Deserialized instructions:");
    for (i, instruction) in deserialized.iter().enumerate() {
        println!("    {}: {}", i, instruction);
    }

    Ok(())
}