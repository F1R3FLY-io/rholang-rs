use anyhow::Result;
use rholang_fake::InterpretationResult;
use shell::providers::{InterpreterProvider, RholangFakeInterpreterProvider};
use std::time::Duration;
use tokio::time::sleep;

// Test for timeout handling in RholangFakeInterpreterProvider.interpret
#[tokio::test]
async fn test_rholang_fake_interpreter_provider_timeout() -> Result<()> {
    let provider = RholangFakeInterpreterProvider::new()?;

    // Set a very long delay to trigger timeout
    provider.set_delay(35000)?; // 35 seconds, which is longer than the 30-second timeout

    // Use a simple valid code that would normally succeed
    let input = "new channel in { @\"stdout\"!(\"Hello, world!\") }";

    // The interpret method should timeout after 30 seconds
    let result = provider.interpret(input).await;

    // Check that we got a timeout error
    match result {
        InterpretationResult::Error(err) => {
            assert!(
                err.to_string().contains("Timeout"),
                "Expected timeout error, got: {}",
                err
            );
        }
        InterpretationResult::Success(_) => {
            panic!("Expected timeout error, got success");
        }
    }

    Ok(())
}

// Test for cancellation handling in RholangFakeInterpreterProvider.interpret
#[tokio::test]
async fn test_rholang_fake_interpreter_provider_cancellation() -> Result<()> {
    let provider = RholangFakeInterpreterProvider::new()?;

    // Set a delay long enough for us to cancel the process
    provider.set_delay(5000)?; // 5 seconds

    // Use a simple valid code that would normally succeed
    let input = "new channel in { @\"stdout\"!(\"Hello, world!\") }";

    // Start the interpretation in a separate task
    let provider_clone = provider.clone();
    let handle = tokio::spawn(async move { provider_clone.interpret(input).await });

    // Give the process time to start
    sleep(Duration::from_millis(100)).await;

    // List processes to get the PID
    let processes = provider.list_processes()?;
    assert!(!processes.is_empty(), "Expected at least one process");

    // Kill the process
    let pid = processes[0].0;
    let killed = provider.kill_process(pid)?;
    assert!(killed, "Failed to kill process {}", pid);

    // Wait for the task to complete
    let result = handle.await?;

    // Check that we got a cancellation error
    match result {
        InterpretationResult::Error(err) => {
            assert!(
                err.to_string().contains("cancelled"),
                "Expected cancellation error, got: {}",
                err
            );
        }
        InterpretationResult::Success(_) => {
            panic!("Expected cancellation error, got success");
        }
    }

    Ok(())
}
