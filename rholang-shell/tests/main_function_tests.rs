use anyhow::Result;
use clap::Parser;
use rholang_shell::{providers::RholangParserInterpreterProvider, run_shell, Args};
use std::time::Duration;
use tokio::time::timeout;

// This test exercises the same code path as the main function.
// The shell may either:
// 1. Complete immediately if stdin is non-TTY (test environment reads empty stdin)
// 2. Block waiting for input if stdin is TTY-like
// Either behavior is acceptable - we just want to verify no panic or error.
#[tokio::test]
async fn test_main_function_code_path() -> Result<()> {
    // Parse empty args (simulating command line with no arguments)
    let args = Args::parse_from(["program_name"]);

    // Create the interpreter provider
    let interpreter = RholangParserInterpreterProvider::new()?;

    // Set a very short delay for tests
    interpreter.set_delay(0)?;

    // Run with a short timeout - both timeout and success are acceptable outcomes
    let result = timeout(Duration::from_millis(100), async {
        run_shell(args, interpreter).await
    })
    .await;

    // Either outcome is fine:
    // - Err: timeout (shell is blocking in interactive mode) - expected
    // - Ok(Ok(_)): shell completed (non-TTY stdin detected) - also fine
    // - Ok(Err(_)): shell error - this would be a failure
    match result {
        Err(_) => {
            // Timeout is expected for interactive mode
        }
        Ok(Ok(_)) => {
            // Completed successfully (non-TTY mode)
        }
        Ok(Err(e)) => {
            panic!("Shell returned an error: {}", e);
        }
    }

    Ok(())
}
