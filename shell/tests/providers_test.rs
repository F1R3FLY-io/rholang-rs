use anyhow::Result;
use rholang_fake::InterpretationResult;
use shell::providers::{
    FakeInterpreterProvider, InterpreterProvider, RholangFakeInterpreterProvider,
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_fake_interpreter_provider() -> Result<()> {
    let provider = FakeInterpreterProvider;

    // Test interpret method
    let input = "1 + 2 * 3";
    let result = provider.interpret(input).await;
    match result {
        InterpretationResult::Success(output) => {
            assert_eq!(output, input);
        }
        InterpretationResult::Error(err) => {
            panic!("Expected success, got error: {}", err);
        }
    }

    // Test list_processes method
    let processes = provider.list_processes()?;
    assert!(processes.is_empty());

    // Test kill_process method
    let killed = provider.kill_process(1)?;
    assert!(!killed);

    // Test kill_all_processes method
    let killed_count = provider.kill_all_processes()?;
    assert_eq!(killed_count, 0);

    Ok(())
}

#[tokio::test]
async fn test_rholang_fake_interpreter_provider_valid_code() -> Result<()> {
    let provider = RholangFakeInterpreterProvider::new()?;
    // Set delay to 0 for tests
    provider.set_delay(0)?;

    // Test interpret method with valid code
    let input = "new channel in { @\"stdout\"!(\"Hello, world!\") }";
    let result = provider.interpret(input).await;
    match result {
        InterpretationResult::Success(output) => {
            assert!(output.contains("Created new name: channel"));
        }
        InterpretationResult::Error(err) => {
            panic!("Expected success, got error: {}", err);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_rholang_fake_interpreter_provider_invalid_code() -> Result<()> {
    let provider = RholangFakeInterpreterProvider::new()?;
    // Set delay to 0 for tests
    provider.set_delay(0)?;

    // Test interpret method with invalid code (missing closing brace)
    let input = "new channel in { @\"stdout\"!(\"Hello, world!\")";
    let result = provider.interpret(input).await;
    assert!(result.is_error());

    Ok(())
}

#[tokio::test]
async fn test_rholang_fake_interpreter_provider_process_management() -> Result<()> {
    let provider = RholangFakeInterpreterProvider::new()?;
    // Set a small delay for this test to ensure processes don't complete too quickly
    provider.set_delay(100)?;

    // Start a long-running process
    let long_running_code = "for (x <- channel) { @\"stdout\"!(x) }";

    // Spawn the process in a separate task so we can continue testing
    let provider_clone = provider.clone();
    tokio::spawn(async move {
        let _ = provider_clone.interpret(long_running_code).await;
    });

    // Give the process time to start
    sleep(Duration::from_millis(100)).await;

    // List processes
    let processes = provider.list_processes()?;
    assert!(!processes.is_empty());

    // Get the process ID
    let pid = processes[0].0;

    // Kill the process
    let killed = provider.kill_process(pid)?;
    assert!(killed);

    // Verify the process is gone
    let processes = provider.list_processes()?;
    assert!(processes.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_rholang_fake_interpreter_provider_kill_all_processes() -> Result<()> {
    let provider = RholangFakeInterpreterProvider::new()?;
    // Set a small delay for this test to ensure processes don't complete too quickly
    provider.set_delay(100)?;

    // Start multiple long-running processes
    let long_running_code1 = "for (x <- channel1) { @\"stdout\"!(x) }";
    let long_running_code2 = "for (y <- channel2) { @\"stdout\"!(y) }";

    // Spawn the processes in separate tasks
    let provider_clone1 = provider.clone();
    tokio::spawn(async move {
        let _ = provider_clone1.interpret(long_running_code1).await;
    });

    let provider_clone2 = provider.clone();
    tokio::spawn(async move {
        let _ = provider_clone2.interpret(long_running_code2).await;
    });

    // Give the processes time to start
    sleep(Duration::from_millis(100)).await;

    // List processes
    let processes = provider.list_processes()?;
    assert_eq!(processes.len(), 2);

    // Kill all processes
    let killed_count = provider.kill_all_processes()?;
    assert_eq!(killed_count, 2);

    // Verify all processes are gone
    let processes = provider.list_processes()?;
    assert!(processes.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_rholang_fake_interpreter_provider_kill_nonexistent_process() -> Result<()> {
    let provider = RholangFakeInterpreterProvider::new()?;
    // Set delay to 0 for tests
    provider.set_delay(0)?;

    // Try to kill a process that doesn't exist
    let killed = provider.kill_process(999)?;
    assert!(!killed);

    Ok(())
}
