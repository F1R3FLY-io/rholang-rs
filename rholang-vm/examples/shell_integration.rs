// Shell Integration Example
// Demonstrates how to use the VM interpreter provider with the shell

use anyhow::Result;
use rholang_vm::interpreter::{InterpreterProvider, RholangVMInterpreterProvider};
use std::io::{self, Write};
use tokio::runtime::Runtime;

fn main() -> Result<()> {
    println!("Rholang VM Shell Integration Example");
    println!("------------------------------------");

    // Create a new VM interpreter provider
    let provider = RholangVMInterpreterProvider::new()?;
    println!("VM interpreter provider created successfully");

    // Create a tokio runtime for async execution
    let runtime = Runtime::new()?;

    // Simple REPL loop
    let mut input = String::new();
    loop {
        print!("rholang> ");
        io::stdout().flush()?;

        input.clear();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "exit" || input == "quit" {
            println!("Exiting...");
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Execute the input using the VM interpreter provider
        let result = runtime.block_on(provider.interpret(input));

        match result {
            rholang_vm::interpreter::InterpretationResult::Success(output) => {
                println!("Result: {}", output);
            }
            rholang_vm::interpreter::InterpretationResult::Error(err) => {
                println!("Error: {}", err);
            }
        }
    }

    // List processes (should be empty at this point)
    let processes = provider.list_processes()?;
    println!("Running processes: {}", processes.len());
    for (pid, code) in processes {
        println!("  Process {}: {}", pid, code);
    }

    Ok(())
}

// Example usage:
// $ cargo run --example shell_integration
// Rholang VM Shell Integration Example
// ------------------------------------
// VM interpreter provider created successfully
// rholang> 1 + 2
// Result: String("1 + 2")
// rholang> new x in { x!(5) | for(y <- x) { y } }
// Result: String("new x in { x!(5) | for(y <- x) { y } }")
// rholang> exit
// Exiting...
// Running processes: 0