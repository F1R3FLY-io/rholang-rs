use std::io::Write;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

mod mock_interpreter;
use mock_interpreter::MockInterpreter;

#[tokio::test]
async fn test_interpreter_receives_commands() -> Result<()> {
    // Create a mock interpreter
    let mock = MockInterpreter::new()
        .with_success_response("command1", "result1")
        .with_success_response("command2", "result2");

    let call_count = Arc::clone(&mock.call_count);
    let calls = Arc::clone(&mock.calls);

    // Call our interpreter
    mock.interpret("command1".to_string()).await?;
    mock.interpret("command2".to_string()).await?;

    // Verify the calls were recorded correctly
    assert_eq!(*call_count.lock().unwrap(), 2);
    assert_eq!(*calls.lock().unwrap(), vec!["command1", "command2"]);

    Ok(())
}

#[tokio::test]
async fn test_interpreter_error_handling() -> Result<()> {
    let mock = MockInterpreter::new()
        .with_error_response("bad_command", "Syntax error");

    let result = mock.interpret("bad_command".to_string()).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Syntax error"));

    Ok(())
}

// This is a simplified version of how main.rs processes commands
// It allows us to test the command processing logic without the full readline interface
async fn process_command(interpreter: impl Interpreter, command: String) -> Result<String> {
    if command == "quit" {
        return Ok("quit".to_string());
    }

    interpreter.interpret(command).await
}

#[tokio::test]
async fn test_process_command() -> Result<()> {
    let mock = MockInterpreter::new()
        .with_success_response("test_cmd", "test_result");

    // Test normal command
    let result = process_command(&mock, "test_cmd".to_string()).await?;
    assert_eq!(result, "test_result");

    // Test quit command
    let result = process_command(&mock, "quit".to_string()).await?;
    assert_eq!(result, "quit");

    // Verify calls
    let calls = mock.get_calls();
    assert_eq!(calls, vec!["test_cmd"]);

    Ok(())
}
