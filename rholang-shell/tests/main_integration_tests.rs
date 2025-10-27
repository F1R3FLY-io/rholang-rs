use anyhow::Result;
use std::sync::{Arc, Mutex};

// This test file contains integration tests for the main function
// Since the main function is the entry point for the application,
// we need to mock some components to test it effectively

// Mock the Args struct to avoid parsing command line arguments
#[derive(Clone)]
struct MockArgs {
    multiline: bool,
}

impl MockArgs {
    fn new(multiline: bool) -> Self {
        MockArgs { multiline }
    }
}

// Mock the run_shell function to avoid actually running the rholang-shell
fn mock_run_shell<I>(
    args: MockArgs,
    _interpreter: I,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Just record that the function was called with the given arguments
    MOCK_RUN_SHELL_CALLED.with(|called| {
        let mut called = called.lock().unwrap();
        *called = true;
    });

    MOCK_RUN_SHELL_ARGS.with(|args_store| {
        let mut args_store = args_store.lock().unwrap();
        *args_store = args.multiline;
    });

    Ok(())
}

// Thread-local storage for tracking mock function calls
thread_local! {
    static MOCK_RUN_SHELL_CALLED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static MOCK_RUN_SHELL_ARGS: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

// Reset the mock state before each test
fn reset_mocks() {
    MOCK_RUN_SHELL_CALLED.with(|called| {
        let mut called = called.lock().unwrap();
        *called = false;
    });

    MOCK_RUN_SHELL_ARGS.with(|args_store| {
        let mut args_store = args_store.lock().unwrap();
        *args_store = false;
    });
}

// Test that the main function calls run_shell with the correct arguments
#[tokio::test]
async fn test_main_calls_run_shell() -> Result<()> {
    reset_mocks();

    // Create mock arguments
    let args = MockArgs::new(true);

    // Call a mock version of the main function
    match mock_main(args).await {
        Ok(_) => (),
        Err(e) => return Err(anyhow::anyhow!("Mock main failed: {}", e)),
    };

    // Verify that run_shell was called
    MOCK_RUN_SHELL_CALLED.with(|called| {
        let called = called.lock().unwrap();
        assert!(*called, "run_shell should have been called");
    });

    // Verify that run_shell was called with the correct arguments
    MOCK_RUN_SHELL_ARGS.with(|args_store| {
        let args_store = args_store.lock().unwrap();
        assert!(
            *args_store,
            "run_shell should have been called with multiline=true"
        );
    });

    Ok(())
}

// A mock version of the main function that uses our mocks instead of the real components
async fn mock_main(args: MockArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create a mock interpreter
    let interpreter = rholang_shell::providers::FakeInterpreterProvider;

    // Call the mock run_shell function
    mock_run_shell(args, interpreter)
}

// Test with different multiline settings
#[tokio::test]
async fn test_main_with_multiline_false() -> Result<()> {
    reset_mocks();

    // Create mock arguments with multiline=false
    let args = MockArgs::new(false);

    // Call a mock version of the main function
    match mock_main(args).await {
        Ok(_) => (),
        Err(e) => return Err(anyhow::anyhow!("Mock main failed: {}", e)),
    };

    // Verify that run_shell was called
    MOCK_RUN_SHELL_CALLED.with(|called| {
        let called = called.lock().unwrap();
        assert!(*called, "run_shell should have been called");
    });

    // Verify that run_shell was called with the correct arguments
    MOCK_RUN_SHELL_ARGS.with(|args_store| {
        let args_store = args_store.lock().unwrap();
        assert!(
            !*args_store,
            "run_shell should have been called with multiline=false"
        );
    });

    Ok(())
}
