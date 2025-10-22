use anyhow::Result;
use clap::Parser;
use shell::{providers::RholangParserInterpreterProvider, run_shell, Args};
use std::time::Duration;
use tokio::time::timeout;

// This test exercises the same code path as the main function
// but with a timeout to prevent it from running indefinitely
// Note: Ignored by default because it requires a TTY (terminal device)
// Run with: cargo test -- --ignored --test-threads=1
#[tokio::test]
#[ignore]
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
        // This will start the rholang-shell and may either timeout or exit cleanly (EOF)
        // We just want to verify that the code path is executed without errors
        run_shell(args, interpreter).await
    })
    .await;

    // We expect either a timeout error OR successful completion (EOF in test environment)
    // Both indicate the shell started correctly
    match result {
        Err(_) => {
            // Timeout - shell is waiting for input (expected in interactive environment)
        }
        Ok(Ok(())) => {
            // Shell exited cleanly (EOF received - expected in test environment)
        }
        Ok(Err(e)) => {
            panic!("Shell returned error: {}", e);
        }
    }

    Ok(())
}

// Test with multiline mode enabled
// Note: Ignored by default because it requires a TTY (terminal device)
// Run with: cargo test -- --ignored --test-threads=1
#[tokio::test]
#[ignore]
async fn test_main_function_with_multiline() -> Result<()> {
    // Parse args with multiline flag
    let args = Args::parse_from(["program_name", "--multiline"]);

    // Verify that multiline mode is enabled
    assert!(args.multiline, "Multiline mode should be enabled");

    // Create the interpreter provider
    let interpreter = RholangParserInterpreterProvider::new()?;

    // Set a very short delay for tests
    interpreter.set_delay(0)?;

    // Run the rholang-shell with a timeout
    let result = timeout(Duration::from_millis(100), async {
        run_shell(args, interpreter).await
    })
    .await;

    // We expect either a timeout error OR successful completion (EOF in test environment)
    // Both indicate the shell started correctly
    match result {
        Err(_) => {
            // Timeout - shell is waiting for input (expected in interactive environment)
        }
        Ok(Ok(())) => {
            // Shell exited cleanly (EOF received - expected in test environment)
        }
        Ok(Err(e)) => {
            panic!("Shell returned error: {}", e);
        }
    }

    Ok(())
}
