// Greeter Contract Example (new rholang-vm)
// This example demonstrates a greeter-like flow using currently supported opcodes.
// Notes:
// - The new VM does not support string literals yet, so we use integer payloads [1] and [2]
//   as stand-ins for greetings.
// - We avoid label/jump control flow since the minimal VM focuses on a linear sequence.

use anyhow::Result;
use rholang_process::Process;
use rholang_vm::api::{Instruction, Opcode, Value};

// RSpace kind code used in tests/examples to separate spaces
const STORE_CONC: u16 = 3;

fn main() -> Result<()> {
    println!("Rholang VM Greeter Contract Example (new)");
    println!("-------------------------------------------");

    // Program outline:
    // new greeter, stdout in {
    //   // approximate greeter calls that send two messages to stdout
    //   stdout!([1]); stdout!([2]);
    //   // then read: remove first, peek second => [2]
    // }

    let program = vec![
        // Create greeter channel (unused in simplified flow, kept for structure)
        Instruction::unary(Opcode::NAME_CREATE, STORE_CONC),
        Instruction::nullary(Opcode::ALLOC_LOCAL),
        Instruction::unary(Opcode::STORE_LOCAL, 0),
        // Create stdout channel
        Instruction::unary(Opcode::NAME_CREATE, STORE_CONC),
        Instruction::nullary(Opcode::ALLOC_LOCAL),
        Instruction::unary(Opcode::STORE_LOCAL, 1),
        // stdout!([1])
        Instruction::unary(Opcode::LOAD_LOCAL, 1),
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, STORE_CONC),
        // stdout!([2])
        Instruction::unary(Opcode::LOAD_LOCAL, 1),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, STORE_CONC),
        // Read: consume first and discard, then peek second
        Instruction::unary(Opcode::LOAD_LOCAL, 1),
        Instruction::unary(Opcode::ASK, STORE_CONC),
        Instruction::nullary(Opcode::POP),
        Instruction::unary(Opcode::LOAD_LOCAL, 1),
        Instruction::unary(Opcode::PEEK, STORE_CONC),
        Instruction::nullary(Opcode::HALT),
    ];

    let mut process = Process::new(program, "greeter_contract: example");
    let result = process.execute()?;

    println!("Final result: {:?}", result);
    assert_eq!(result, Value::List(vec![Value::Int(2)]));

    println!("Example completed successfully.");
    Ok(())
}
