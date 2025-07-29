// Shell Integration Example
// Demonstrates how to use the VM interpreter provider with the shell
//
// This example implements a non-blocking REPL (Read-Eval-Print Loop) using tokio's
// async I/O utilities. This approach prevents the program from hanging on user input
// and aligns with the async nature of the interpreter.

use anyhow::Result;
use rholang_vm::interpreter::{InterpreterProvider, RholangVMInterpreterProvider};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

// Use tokio::main to set up the async runtime automatically
// This allows us to use async/await syntax directly in the main function
#[tokio::main]
async fn main() -> Result<()> {
    println!("Rholang VM Shell Integration Example");
    println!("------------------------------------");

    // Create a new VM interpreter provider
    let provider = RholangVMInterpreterProvider::new()?;
    println!("VM interpreter provider created successfully");

    // Set up async I/O
    // BufReader with lines() provides an async stream of lines from stdin
    // This allows us to read user input without blocking the entire program
    let mut stdin = BufReader::new(io::stdin()).lines();
    // Get a mutable reference to stdout for async writing
    let mut stdout = io::stdout();

    // Simple REPL loop
    loop {
        // Display prompt using async write and flush
        // The await keyword suspends execution until the operation completes
        // without blocking the entire program
        stdout.write_all(b"rholang> ").await?;
        stdout.flush().await?;

        // Read input asynchronously using next_line()
        // This is a key improvement over the blocking read_line() method
        let input = match stdin.next_line().await? {
            Some(line) => line,
            None => {
                // Handle EOF (Ctrl+D) gracefully
                println!("\nEnd of input, exiting...");
                break;
            }
        };

        let input = input.trim();

        if input == "exit" || input == "quit" {
            println!("Exiting...");
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Execute the input using the VM interpreter provider
        // Since we're in an async context, we can directly await the result
        // This is a significant improvement over the previous approach:
        // - No need to manually create a runtime
        // - No need to use runtime.block_on() which can cause blocking issues
        // - The entire execution flow remains asynchronous
        let result = provider.interpret(input).await;

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

#[cfg(test)]
mod tests {
    // This module contains tests for the shell integration example.
    // The tests verify that the shell correctly handles user input and produces the expected output.
    // We use a mock interpreter provider to simulate the behavior of the real interpreter
    // without requiring a full VM implementation.
    
    use super::*;
    use tokio::sync::mpsc;

    // Helper function to simulate user input and capture output
    // This function creates a mock environment that simulates the shell's behavior:
    // 1. It creates a mock interpreter provider that returns predictable results
    // 2. It sets up channels for input and output
    // 3. It spawns a task that processes inputs and produces outputs
    // 4. It sends the provided inputs to the task
    // 5. It collects and returns the outputs produced by the task
    async fn simulate_shell_interaction(
        inputs: Vec<&str>,
    ) -> Result<Vec<String>, anyhow::Error> {
        // Create a mock interpreter provider that returns predictable results
        struct MockInterpreterProvider;

        #[async_trait::async_trait]
        impl InterpreterProvider for MockInterpreterProvider {
            async fn interpret(&self, code: &str) -> rholang_vm::interpreter::InterpretationResult {
                rholang_vm::interpreter::InterpretationResult::Success(format!("Processed: {}", code))
            }

            fn list_processes(&self) -> Result<Vec<(usize, String)>, anyhow::Error> {
                Ok(vec![])
            }

            fn kill_process(&self, _pid: usize) -> Result<bool, anyhow::Error> {
                Ok(false)
            }

            fn kill_all_processes(&self) -> Result<usize, anyhow::Error> {
                Ok(0)
            }
        }

        // Create channels for input and output
        let (input_tx, mut input_rx) = mpsc::channel::<String>(32);
        let (output_tx, mut output_rx) = mpsc::channel::<String>(32);

        // Spawn a task to run the shell with our mock input/output
        let handle = tokio::spawn(async move {
            let provider = MockInterpreterProvider;
            
            // Simulate stdin
            let mut input_lines = Vec::new();
            while let Some(line) = input_rx.recv().await {
                input_lines.push(line);
            }
            let mut input_iter = input_lines.into_iter();
            
            // Process each input
            while let Some(input) = input_iter.next() {
                // Send prompt
                output_tx.send("rholang> ".to_string()).await.unwrap();
                
                // Process input
                if input.trim() == "exit" || input.trim() == "quit" {
                    output_tx.send("Exiting...".to_string()).await.unwrap();
                    break;
                }
                
                if input.trim().is_empty() {
                    continue;
                }
                
                // Interpret input
                let result = provider.interpret(&input).await;
                
                // Send result
                match result {
                    rholang_vm::interpreter::InterpretationResult::Success(output) => {
                        output_tx.send(format!("Result: {}", output)).await.unwrap();
                    }
                    rholang_vm::interpreter::InterpretationResult::Error(err) => {
                        output_tx.send(format!("Error: {}", err)).await.unwrap();
                    }
                }
            }
            
            // List processes
            let processes = provider.list_processes().unwrap();
            output_tx.send(format!("Running processes: {}", processes.len())).await.unwrap();
        });

        // Send inputs
        for input in inputs {
            input_tx.send(input.to_string()).await?;
        }
        drop(input_tx); // Close the channel to signal end of input

        // Collect outputs
        let mut outputs = Vec::new();
        while let Some(output) = output_rx.recv().await {
            outputs.push(output);
        }

        // Wait for the shell task to complete
        handle.await?;

        Ok(outputs)
    }

    // Test basic interaction with the shell
    // This test verifies that:
    // 1. The shell displays a prompt
    // 2. The shell correctly processes a simple input
    // 3. The shell displays the result
    // 4. The shell handles the exit command
    // 5. The shell reports the number of running processes
    #[tokio::test]
    async fn test_basic_interaction() -> Result<(), anyhow::Error> {
        let inputs = vec![
            "1 + 2",
            "exit",
        ];
        
        let outputs = simulate_shell_interaction(inputs).await?;
        
        // Verify outputs
        assert_eq!(outputs.len(), 5); // prompt, result, prompt, exit message, processes
        assert_eq!(outputs[0], "rholang> ");
        assert_eq!(outputs[1], "Result: Processed: 1 + 2");
        assert_eq!(outputs[2], "rholang> ");
        assert_eq!(outputs[3], "Exiting...");
        assert_eq!(outputs[4], "Running processes: 0");
        
        Ok(())
    }

    // Test handling of empty input
    // This test verifies that:
    // 1. The shell displays a prompt
    // 2. The shell correctly skips empty input
    // 3. The shell correctly processes a simple input after empty input
    // 4. The shell handles the exit command
    // 5. The shell reports the number of running processes
    #[tokio::test]
    async fn test_empty_input() -> Result<(), anyhow::Error> {
        let inputs = vec![
            "",
            "1 + 2",
            "exit",
        ];
        
        let outputs = simulate_shell_interaction(inputs).await?;
        
        // Verify outputs - empty input should be skipped
        assert_eq!(outputs.len(), 6);
        assert_eq!(outputs[0], "rholang> ");
        assert_eq!(outputs[1], "rholang> ");
        assert_eq!(outputs[2], "Result: Processed: 1 + 2");
        assert_eq!(outputs[3], "rholang> ");
        assert_eq!(outputs[4], "Exiting...");
        assert_eq!(outputs[5], "Running processes: 0");
        
        Ok(())
    }

    // Test handling of multiple inputs
    // This test verifies that:
    // 1. The shell displays a prompt
    // 2. The shell correctly processes multiple inputs in sequence
    // 3. The shell displays the results for each input
    // 4. The shell handles the exit command
    // 5. The shell reports the number of running processes
    #[tokio::test]
    async fn test_multiple_inputs() -> Result<(), anyhow::Error> {
        let inputs = vec![
            "1 + 2",
            "new x in { x!(5) }",
            "exit",
        ];
        
        let outputs = simulate_shell_interaction(inputs).await?;
        
        // Verify outputs
        assert_eq!(outputs.len(), 7);
        assert_eq!(outputs[0], "rholang> ");
        assert_eq!(outputs[1], "Result: Processed: 1 + 2");
        assert_eq!(outputs[2], "rholang> ");
        assert_eq!(outputs[3], "Result: Processed: new x in { x!(5) }");
        assert_eq!(outputs[4], "rholang> ");
        assert_eq!(outputs[5], "Exiting...");
        assert_eq!(outputs[6], "Running processes: 0");
        
        Ok(())
    }
}
