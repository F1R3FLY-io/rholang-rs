use anyhow::Result;
use clap::Parser;
use rholang_shell::{providers::RholangParserInterpreterProvider, run_shell, Args};
use std::time::Duration;
use tokio::time::timeout;

// This test exercises the same code path as the main function
// but with a timeout to prevent it from running indefinitely
#[tokio::test]
async fn test_main_function_code_path() -> Result<()> {
    // Parse empty args (simulating command line with no arguments)
    let args = Args::parse_from(["program_name"]);

    // Create the interpreter provider
    let interpreter = RholangParserInterpreterProvider::new()?;

    // Set a very short delay for tests
    interpreter.set_delay(0)?;

    // Run the rholang-shell with a timeout to prevent it from running indefinitely
    // We're not actually testing the rholang-shell's functionality here,
    // just that the code path doesn't panic or error out
    let result = timeout(Duration::from_millis(100), async {
        // This will start the rholang-shell and immediately time out
        // We just want to verify that the code path is executed without errors
        run_shell(args, interpreter).await
    })
    .await;

    // We expect a timeout error, which is fine
    assert!(result.is_err(), "Expected timeout error");

    Ok(())
}
