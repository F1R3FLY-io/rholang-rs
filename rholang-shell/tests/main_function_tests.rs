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

    // Run the rholang-shell with a timeout to prevent hangs.
    // In non-interactive test environments, the shell may exit quickly with EOF
    // or an input-device error instead of blocking for user input.
    let result = timeout(Duration::from_millis(100), async {
        run_shell(args, interpreter).await
    })
    .await;

    // The important assertion for this test is that the code path does not hang.
    assert!(result.is_ok(), "run_shell unexpectedly timed out");

    Ok(())
}
