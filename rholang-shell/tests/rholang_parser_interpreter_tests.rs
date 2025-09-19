use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;

use shell::providers::{InterpreterProvider, RholangParserInterpreterProvider};

#[tokio::test]
async fn test_rholang_parser_interpreter_creation() -> Result<()> {
    // Test that we can create a new RholangParserInterpreterProvider
    let interpreter = RholangParserInterpreterProvider::new()?;

    // Test that we can set a delay
    interpreter.set_delay(100)?;

    Ok(())
}

#[tokio::test]
async fn test_rholang_parser_interpreter_with_valid_code() -> Result<()> {
    let interpreter = RholangParserInterpreterProvider::new()?;

    // Use a simple valid Rholang code
    let input = "new x in { x!(5) }";
    let result = interpreter.interpret(input).await;

    assert!(result.is_success());

    Ok(())
}

#[tokio::test]
async fn test_rholang_parser_interpreter_with_invalid_code() -> Result<()> {
    let interpreter = RholangParserInterpreterProvider::new()?;

    // Use invalid Rholang code
    let input = "new x in { x!(5) }}}"; // Extra closing braces
    let result = interpreter.interpret(input).await;

    assert!(result.is_error());

    Ok(())
}

#[tokio::test]
async fn test_rholang_parser_interpreter_process_management() -> Result<()> {
    let interpreter = RholangParserInterpreterProvider::new()?;

    // Set a delay to ensure the process stays running long enough for us to check
    interpreter.set_delay(500)?;

    // Start a process
    let handle = tokio::spawn({
        let interpreter = interpreter.clone();
        async move {
            let input = "new x in { x!(5) }";
            interpreter.interpret(input).await
        }
    });

    // Give it a moment to start
    sleep(Duration::from_millis(100)).await;

    // List processes
    let processes = interpreter.list_processes()?;
    assert!(
        !processes.is_empty(),
        "Expected at least one running process"
    );

    // Get the process ID
    let pid = processes[0].0;

    // Kill the process
    let killed = interpreter.kill_process(pid)?;
    assert!(killed, "Expected process to be killed");

    // Wait for the handle to complete
    let result = handle.await?;
    assert!(result.is_error(), "Expected error due to cancellation");

    // List processes again to verify it's gone
    let processes = interpreter.list_processes()?;
    assert!(processes.is_empty(), "Expected no running processes");

    Ok(())
}

#[tokio::test]
async fn test_rholang_parser_interpreter_kill_all_processes() -> Result<()> {
    let interpreter = RholangParserInterpreterProvider::new()?;

    // Set a delay to ensure processes stay running
    interpreter.set_delay(1000)?;

    // Start multiple processes
    let handle1 = tokio::spawn({
        let interpreter = interpreter.clone();
        async move {
            let input = "new x in { x!(1) }";
            interpreter.interpret(input).await
        }
    });

    let handle2 = tokio::spawn({
        let interpreter = interpreter.clone();
        async move {
            let input = "new y in { y!(2) }";
            interpreter.interpret(input).await
        }
    });

    // Give them a moment to start
    sleep(Duration::from_millis(100)).await;

    // List processes to verify they're running
    let processes = interpreter.list_processes()?;
    assert_eq!(processes.len(), 2, "Expected two running processes");

    // Kill all processes
    let killed_count = interpreter.kill_all_processes()?;
    assert_eq!(killed_count, 2, "Expected two processes to be killed");

    // Wait for handles to complete
    let result1 = handle1.await?;
    let result2 = handle2.await?;

    assert!(result1.is_error(), "Expected error due to cancellation");
    assert!(result2.is_error(), "Expected error due to cancellation");

    // List processes again to verify they're gone
    let processes = interpreter.list_processes()?;
    assert!(processes.is_empty(), "Expected no running processes");

    Ok(())
}
