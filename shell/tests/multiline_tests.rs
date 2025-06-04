use anyhow::Result;
use tokio;

use shell::multiline_helper::{has_double_newline, process_line};
use shell::interpreter::{FakeInterpreter, Interpreter};

#[tokio::test]
async fn test_multiline_code_execution() -> Result<()> {
    let interpreter = FakeInterpreter;

    // Create a multiline string
    let multiline_code = "fn main() {\n    println!(\"Hello, World!\");\n}";

    // Test interpreting a multiline string directly
    let result = interpreter.interpret(multiline_code.to_string()).await?;
    assert_eq!(result, multiline_code);

    // Test the line-by-line accumulation process
    let mut buffer = String::new();
    let mut in_multiline_mode = false;

    // First line
    let line1 = "fn main() {";
    let execute = process_line(line1, &mut buffer, &mut in_multiline_mode);
    assert!(!execute, "Should not execute yet");
    assert!(!in_multiline_mode, "Should not be in multiline mode yet");

    // Empty line to enter multiline mode
    let execute = process_line("", &mut buffer, &mut in_multiline_mode);
    assert!(!execute, "Should not execute yet");
    assert!(in_multiline_mode, "Should be in multiline mode now");

    // Second line
    let line2 = "    println!(\"Hello, World!\");";
    let execute = process_line(line2, &mut buffer, &mut in_multiline_mode);
    assert!(!execute, "Should not execute yet");
    assert!(in_multiline_mode, "Should still be in multiline mode");

    // Third line
    let line3 = "}";
    let execute = process_line(line3, &mut buffer, &mut in_multiline_mode);
    assert!(!execute, "Should not execute yet");
    assert!(in_multiline_mode, "Should still be in multiline mode");

    // Empty line to trigger execution
    let execute = process_line("", &mut buffer, &mut in_multiline_mode);
    assert!(execute, "Should be ready to execute");
    assert!(!in_multiline_mode, "Should no longer be in multiline mode");

    // Check that the buffer has the correct content
    assert_eq!(buffer, "fn main() {\n\n    println!(\"Hello, World!\");\n}");

    // Test that the interpreter would handle this buffer correctly
    let result = interpreter.interpret(buffer.clone()).await?;
    assert_eq!(result, buffer);

    Ok(())
}

#[tokio::test]
async fn test_multiline_with_cancel() -> Result<()> {
    // Test cancelling multiline input
    let mut buffer = String::new();
    let mut in_multiline_mode = false;

    // Start with some input
    process_line("let x = 5;", &mut buffer, &mut in_multiline_mode);
    process_line("", &mut buffer, &mut in_multiline_mode);
    process_line("let y = 10;", &mut buffer, &mut in_multiline_mode);

    // Verify we're in multiline mode with content
    assert!(in_multiline_mode);
    assert!(!buffer.is_empty());

    // Simulate cancel operation (Ctrl+C in the REPL)
    buffer.clear();
    in_multiline_mode = false;

    // Verify state is reset
    assert!(!in_multiline_mode);
    assert!(buffer.is_empty());

    Ok(())
}
