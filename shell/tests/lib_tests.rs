use anyhow::Result;
use std::io::Cursor;

use shell::{
    handle_interrupt, process_multiline_input, process_single_line_input, process_special_command,
    providers::FakeInterpreterProvider,
};

// Helper function to create a fake interpreter provider
fn create_fake_interpreter() -> FakeInterpreterProvider {
    FakeInterpreterProvider
}

#[tokio::test]
async fn test_process_special_command_help() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".help",
        &mut buffer,
        &mut multiline,
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
async fn test_process_special_command_mode() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".mode",
        &mut buffer,
        &mut multiline,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit, "Mode command should not exit");
    assert!(!multiline, "Multiline mode should be toggled");

    // Reset cursor position to read output
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;

    assert!(
        output.contains("Switched to single line mode"),
        "Mode switch message not displayed"
    );

    Ok(())
}

#[tokio::test]
async fn test_process_special_command_mode_to_multiline() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = false; // Start in single line mode
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".mode",
        &mut buffer,
        &mut multiline,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit, "Mode command should not exit");
    assert!(multiline, "Multiline mode should be toggled");

    // Reset cursor position to read output
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;

    assert!(
        output.contains("Switched to multiline mode"),
        "Mode switch message not displayed"
    );

    Ok(())
}

#[tokio::test]
async fn test_process_special_command_quit() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".quit",
        &mut buffer,
        &mut multiline,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(should_exit, "Quit command should exit");

    // Reset cursor position to read output
    stdout.set_position(0);
    let output = String::from_utf8(stdout.into_inner())?;

    assert!(
        output.contains("Exiting shell..."),
        "Exit message not displayed"
    );

    Ok(())
}

#[tokio::test]
async fn test_process_special_command_list() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];
    let mut multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".list",
        &mut buffer,
        &mut multiline,
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
    let mut multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".delete",
        &mut buffer,
        &mut multiline,
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
    let mut multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".delete",
        &mut buffer,
        &mut multiline,
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
    let mut multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".reset",
        &mut buffer,
        &mut multiline,
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
    let mut multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".buffer",
        &mut buffer,
        &mut multiline,
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
    let mut multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        ".unknown",
        &mut buffer,
        &mut multiline,
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
    let mut multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    let should_exit = process_special_command(
        "not_special",
        &mut buffer,
        &mut multiline,
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

    let command = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;

    assert!(command.is_some(), "Empty line should produce a command");
    assert_eq!(
        command.unwrap(),
        "line1\nline2",
        "Command should be all lines joined with newlines"
    );
    assert!(buffer.is_empty(), "Buffer should be cleared");

    Ok(())
}

#[tokio::test]
async fn test_process_single_line_input_empty() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = false;

    let command =
        process_single_line_input("".to_string(), &mut buffer, &mut multiline, |_| Ok(()))?;

    assert!(command.is_none(), "Empty line should not produce a command");
    assert!(buffer.is_empty(), "Buffer should remain empty");
    assert!(!multiline, "Mode should remain single line");

    Ok(())
}

#[tokio::test]
async fn test_process_single_line_input_with_content() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = false;

    let command = process_single_line_input(
        "let x = 10;".to_string(),
        &mut buffer,
        &mut multiline,
        |_| Ok(()),
    )?;

    assert!(command.is_some(), "Non-empty line should produce a command");
    assert_eq!(
        command.unwrap(),
        "let x = 10;",
        "Command should be the input line"
    );
    assert!(buffer.is_empty(), "Buffer should remain empty");
    assert!(!multiline, "Mode should remain single line");

    Ok(())
}

#[tokio::test]
async fn test_process_single_line_input_with_brackets() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = false;

    let command = process_single_line_input(
        "for (x <- y) {".to_string(),
        &mut buffer,
        &mut multiline,
        |_| Ok(()),
    )?;

    assert!(
        command.is_none(),
        "Line ending inside brackets should not produce a command"
    );
    assert_eq!(buffer.len(), 1, "Buffer should have one item");
    assert_eq!(
        buffer[0], "for (x <- y) {",
        "Buffer should contain the input line"
    );
    assert!(multiline, "Mode should switch to multiline");

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
        multiline,
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
async fn test_handle_interrupt_single_line() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];
    let multiline = false;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = create_fake_interpreter();

    handle_interrupt(
        &mut buffer,
        multiline,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert_eq!(
        buffer.len(),
        2,
        "Buffer should not be cleared in single line mode"
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
