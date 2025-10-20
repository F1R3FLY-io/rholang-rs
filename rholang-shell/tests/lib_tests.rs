use anyhow::Result;
use std::io::Cursor;

use rholang_shell::{
    handle_interrupt, process_multiline_input, process_special_command,
    providers::FakeInterpreterProvider,
};

// Helper function to create a fake interpreter provider
fn create_fake_interpreter() -> FakeInterpreterProvider {
    FakeInterpreterProvider
}

#[tokio::test]
async fn test_process_special_command_help() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".help",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit, "Help command should not exit");

    // Reset cursor position to read output
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;

    assert!(
        output.contains("Available commands:"),
        "Help message not displayed"
    );

    Ok(())
}



#[tokio::test]
async fn test_process_special_command_quit() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".quit",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(should_exit, "Quit command should exit");

    // Reset cursor position to read output
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;

    assert!(
        output.contains("Exiting rholang-shell..."),
        "Exit message not displayed"
    );

    Ok(())
}

#[tokio::test]
async fn test_process_special_command_list() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".list",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit, "List command should not exit");

    // Reset cursor position to read output
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;

    assert!(
        output.contains("Edited lines:"),
        "List header not displayed"
    );
    assert!(output.contains("line1"), "First line not in list output");
    assert!(output.contains("line2"), "Second line not in list output");

    Ok(())
}

#[tokio::test]
async fn test_process_special_command_delete() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".delete",
        &mut buffer,
        
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit, "Delete command should not exit");
    assert_eq!(buffer.len(), 1, "Buffer should have one item left");
    assert_eq!(buffer[0], "line1", "First line should remain");

    // Reset cursor position to read output
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;

    assert!(
        output.contains("Removed last line: line2"),
        "Delete message not displayed"
    );

    Ok(())
}

#[tokio::test]
async fn test_process_special_command_delete_empty() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".delete",
        &mut buffer,
        
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit, "Delete command should not exit");

    // Reset cursor position to read output
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;

    assert!(
        output.contains("Buffer is empty, nothing to delete"),
        "Empty buffer message not displayed"
    );

    Ok(())
}

#[tokio::test]
async fn test_process_special_command_reset() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".reset",
        &mut buffer,
        
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit, "Reset command should not exit");
    assert!(buffer.is_empty(), "Buffer should be empty after reset");

    // Reset cursor position to read output
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;

    assert!(
        output.contains("Buffer reset"),
        "Reset message not displayed"
    );

    Ok(())
}

#[tokio::test]
async fn test_process_special_command_buffer() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".buffer",
        &mut buffer,
        
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit, "Buffer command should not exit");

    // Reset cursor position to read output
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;

    assert!(
        output.contains("Current buffer:"),
        "Buffer header not displayed"
    );
    assert!(output.contains("line1"), "First line not in buffer output");
    assert!(output.contains("line2"), "Second line not in buffer output");

    Ok(())
}

#[tokio::test]
async fn test_process_special_command_unknown() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".unknown",
        &mut buffer,
        
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit, "Unknown command should not exit");

    // Reset cursor position to read output
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;

    assert!(
        output.contains("Unknown command: .unknown"),
        "Unknown command message not displayed"
    );

    Ok(())
}

#[tokio::test]
async fn test_process_special_command_not_special() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        "not_special",
        &mut buffer,
        
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit, "Non-special command should not exit");
    assert_eq!(
        stdout.get_ref().len(),
        0,
        "No output should be produced for non-special commands"
    );

    Ok(())
}

#[tokio::test]
async fn test_process_multiline_input_empty_buffer_empty_line() -> Result<()> {
    let mut buffer = Vec::new();

    let command = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;

    assert!(command.is_none(), "Empty line should not produce a command");
    assert!(buffer.is_empty(), "Buffer should remain empty");

    Ok(())
}

#[tokio::test]
async fn test_process_multiline_input_empty_buffer_with_line() -> Result<()> {
    let mut buffer = Vec::new();

    let command = process_multiline_input("line1".to_string(), &mut buffer, |_| Ok(()))?;

    assert!(command.is_none(), "First line should not produce a command");
    assert_eq!(buffer.len(), 1, "Buffer should have one item");
    assert_eq!(buffer[0], "line1", "Buffer should contain the input line");

    Ok(())
}

#[tokio::test]
async fn test_process_multiline_input_add_line() -> Result<()> {
    let mut buffer = vec!["line1".to_string()];

    let command = process_multiline_input("line2".to_string(), &mut buffer, |_| Ok(()))?;

    assert!(
        command.is_none(),
        "Adding a line should not produce a command"
    );
    assert_eq!(buffer.len(), 2, "Buffer should have two items");
    assert_eq!(buffer[0], "line1", "First line should be preserved");
    assert_eq!(buffer[1], "line2", "Second line should be added");

    Ok(())
}

#[tokio::test]
async fn test_process_multiline_input_execute() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];

    // First empty line should be ignored
    let first = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;
    assert!(first.is_none(), "First empty line should not execute");

    // Second consecutive empty line should execute
    let command = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;

    assert!(command.is_some(), "Second empty line should produce a command");
    assert_eq!(
        command.unwrap(),
        "line1\nline2",
        "Command should be all lines joined with newlines"
    );
    assert_eq!(buffer, vec!["line1".to_string(), "line2".to_string()], "Buffer should be kept after execution");

    Ok(())
}

#[tokio::test]
async fn test_process_multiline_input_open_bracket_not_execute() -> Result<()> {
    // Buffer with an unmatched opening bracket should not execute on empty line
    let mut buffer = vec!["for (x <- y) {".to_string()];

    let command = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;

    assert!(command.is_none(), "Should not execute when brackets are open");
    assert_eq!(buffer.len(), 1, "Buffer should remain with the open line");

    // Now close the bracket. First empty should be ignored, second should execute
    let _ = process_multiline_input("}".to_string(), &mut buffer, |_| Ok(()))?;
    let first_empty = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;
    assert!(first_empty.is_none(), "First empty after balancing should not execute");
    let command2 = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;

    assert!(command2.is_some(), "Second empty should execute after brackets are balanced");
    assert_eq!(
        command2.unwrap(),
        "for (x <- y) {\n}",
        "Command should include both lines"
    );
    assert_eq!(buffer, vec!["for (x <- y) {".to_string(), "}".to_string()], "Buffer should be kept after execution");

    Ok(())
}


#[tokio::test]
async fn test_handle_interrupt_multiline() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];
    let multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    handle_interrupt(
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(
        buffer.is_empty(),
        "Buffer should be cleared in multiline mode"
    );

    // Reset cursor position to read output
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;

    assert!(
        output.contains("Input interrupted with Ctrl+C"),
        "Interrupt message not displayed"
    );

    Ok(())
}



#[tokio::test]
async fn test_process_special_command_load_success() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    // Use an existing corpus file in the repository (path relative to this crate)
    let path = "../rholang-parser/tests/corpus/bank_contract.rho";
    let cmd = format!(".load {}", path);

    let should_exit = process_special_command(
        &cmd,
        &mut buffer,
        
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit, ".load should not exit");
    assert!(!buffer.is_empty(), "Buffer should be populated after loading a file");

    // Verify output mentions loading
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Loaded"), "Output should confirm loading");

    Ok(())
}

#[tokio::test]
async fn test_process_special_command_load_usage() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".load",
        &mut buffer,
        
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit, ".load with no args should not exit");
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Usage: .load <file>"));
    Ok(())
}

#[tokio::test]
async fn test_process_special_command_load_nonexistent() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".load /no/such/file/definitely_missing.rho",
        &mut buffer,
        
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit, ".load nonexistent should not exit");
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Error loading file"));
    Ok(())
}


#[tokio::test]
async fn test_process_multiline_input_open_square_bracket_not_execute() -> Result<()> {
    // Buffer with an unmatched opening square bracket should not execute on empty line
    let mut buffer = vec!["let x = [1, 2, 3".to_string()];

    let command = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;

    assert!(command.is_none(), "Should not execute when square bracket is open");
    assert_eq!(buffer.len(), 1, "Buffer should remain with the open line");

    // Now close the square bracket. First empty should be ignored, second should execute
    let _ = process_multiline_input("]".to_string(), &mut buffer, |_| Ok(()))?;
    let first_empty = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;
    assert!(first_empty.is_none(), "First empty after balancing should not execute");
    let command2 = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;

    assert!(command2.is_some(), "Second empty should execute after square bracket is closed");
    assert_eq!(
        command2.unwrap(),
        "let x = [1, 2, 3\n]",
        "Command should include both lines"
    );
    assert_eq!(buffer, vec!["let x = [1, 2, 3".to_string(), "]".to_string()], "Buffer should be kept after execution");

    Ok(())
}

#[tokio::test]
async fn test_process_multiline_input_mixed_brackets_all_types() -> Result<()> {
    // Mixed brackets: ensure we only execute when all types are balanced ((), [], {})
    let mut buffer = vec!["A [B {C".to_string()];

    // Empty line should NOT execute because brackets are unbalanced
    let command = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;
    assert!(command.is_none(), "Should not execute when mixed brackets are open");

    // Close both curly and square
    let _ = process_multiline_input("}]".to_string(), &mut buffer, |_| Ok(()))?;

    // Now, the first empty should be ignored and the second should execute
    let first_empty = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;
    assert!(first_empty.is_none(), "First empty after balancing should not execute");
    let command2 = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;
    assert!(command2.is_some(), "Second empty should execute after all brackets are balanced");

    let expected = "A [B {C\n}]".to_string();
    assert_eq!(command2.unwrap(), expected);
    assert_eq!(buffer, vec!["A [B {C".to_string(), "}]".to_string()], "Buffer should be kept after execution");

    Ok(())
}
