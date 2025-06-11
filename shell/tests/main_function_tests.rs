use anyhow::Result;
use clap::Parser;
use shell::{providers::RholangFakeInterpreterProvider, run_shell, Args};
use std::time::Duration;
use tokio::time::timeout;

// This test exercises the same code path as the main function
// but with a timeout to prevent it from running indefinitely
#[tokio::test]
async fn test_main_function_code_path() -> Result<()> {
    // Parse empty args (simulating command line with no arguments)
    let args = Args::parse_from(["program_name"]);

    // Create the interpreter provider
    let interpreter = RholangFakeInterpreterProvider::new()?;

    // Set a very short delay for tests
    interpreter.set_delay(0)?;

    // Run the shell with a timeout to prevent it from running indefinitely
    // We're not actually testing the shell's functionality here,
    // just that the code path doesn't panic or error out
    let result = timeout(Duration::from_millis(100), async {
        // This will start the shell and immediately time out
        // We just want to verify that the code path is executed without errors
        run_shell(args, interpreter).await
    })
    .await;

    // We expect a timeout error, which is fine
    assert!(result.is_err(), "Expected timeout error");

    Ok(())
}

// Test with multiline mode enabled
#[tokio::test]
async fn test_main_function_with_multiline() -> Result<()> {
    // Parse args with multiline flag
    let args = Args::parse_from(["program_name", "--multiline"]);

    // Verify that multiline mode is enabled
    assert!(args.multiline, "Multiline mode should be enabled");

    // Create the interpreter provider
    let interpreter = RholangFakeInterpreterProvider::new()?;

    // Set a very short delay for tests
    interpreter.set_delay(0)?;

    // Run the shell with a timeout
    let result = timeout(Duration::from_millis(100), async {
        run_shell(args, interpreter).await
    })
    .await;

    // We expect a timeout error, which is fine
    assert!(result.is_err(), "Expected timeout error");

    Ok(())
}
