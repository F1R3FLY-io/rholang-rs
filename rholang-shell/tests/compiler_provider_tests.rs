use anyhow::Result;
use rholang_shell::providers::{
    InterpretationResult, InterpreterProvider, RholangCompilerInterpreterProvider,
};

// Use Tokio tests for async provider methods

#[tokio::test]
async fn interpret_success_nil() -> Result<()> {
    let provider = RholangCompilerInterpreterProvider::new()?;
    match provider.interpret("Nil").await {
        InterpretationResult::Success(s) => assert_eq!(s.trim(), "Nil"),
        other => panic!("Expected Success, got: {:?}", other),
    }
    Ok(())
}

#[tokio::test]
async fn interpret_parse_error() -> Result<()> {
    let provider = RholangCompilerInterpreterProvider::new()?;
    match provider.interpret("(").await {
        InterpretationResult::Error(e) => {
            // Error message should include cleaned parsing info (without SourcePos spam)
            assert!(!e.message.is_empty());
        }
        other => panic!("Expected Error, got: {:?}", other),
    }
    Ok(())
}

#[tokio::test]
async fn process_management_and_cancellation() -> Result<()> {
    let provider = RholangCompilerInterpreterProvider::new()?;
    // Add artificial delay to make the task cancellable
    provider.set_delay(200)?; // 200ms

    // Start interpretation in background
    let provider_clone = provider.clone();
    let handle = tokio::spawn(async move { provider_clone.interpret("Nil").await });

    // Give it a moment to register the process
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    // List processes and kill the first one if present
    let procs = provider.list_processes()?;
    if let Some((pid, _code)) = procs.first().cloned() {
        let killed = provider.kill_process(pid)?;
        assert!(killed, "Expected kill_process to return true");
    }

    let result = handle.await.expect("join handle");
    match result {
        InterpretationResult::Error(e) => {
            assert!(e.message.contains("cancelled") || e.message.contains("cancel"));
        }
        other => panic!("Expected cancellation Error, got: {:?}", other),
    }

    // After cancellation, there should be no running processes
    assert!(provider.list_processes()?.is_empty());

    Ok(())
}
