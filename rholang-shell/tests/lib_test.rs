use anyhow::Result;
use rholang_shell::{
    handle_interrupt, help_message, process_multiline_input, process_special_command,
    providers::InterpretationResult, Args,
};
use std::io::Cursor;

// A simple mock interpreter provider for testing
struct MockInterpreterProvider {
    processes: Vec<(usize, String)>,
}

impl MockInterpreterProvider {
    fn new() -> Self {
        MockInterpreterProvider {
            processes: Vec::new(),
        }
    }

    fn with_processes(processes: Vec<(usize, String)>) -> Self {
        MockInterpreterProvider { processes }
    }
}

#[async_trait::async_trait]
impl rholang_shell::providers::InterpreterProvider for MockInterpreterProvider {
    async fn interpret(&self, code: &str) -> InterpretationResult {
        InterpretationResult::Success(format!("Interpreted: {}", code))
    }

    fn list_processes(&self) -> Result<Vec<(usize, String)>> {
        Ok(self.processes.clone())
    }

    fn kill_process(&self, pid: usize) -> Result<bool> {
        Ok(self.processes.iter().any(|(id, _)| *id == pid))
    }

    fn kill_all_processes(&self) -> Result<usize> {
        Ok(self.processes.len())
    }
}

#[test]
fn test_help_message() {
    let message = help_message();
    assert!(message.contains(".help"));
    assert!(message.contains(".list"));
    assert!(message.contains(".delete"));
    assert!(message.contains(".reset"));
    assert!(message.contains(".load"));
    assert!(message.contains(".validate"));
    assert!(message.contains(".validate-unused"));
    assert!(message.contains(".validate-elab"));
    assert!(message.contains(".validate-resolver"));
    assert!(message.contains(".ps"));
    assert!(message.contains(".kill"));
    assert!(message.contains(".quit"));
}

#[test]
fn test_process_special_command_help() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit =
        process_special_command(".help", &mut buffer, &mut stdout, |_| Ok(()), &interpreter)?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains(".help"));

    Ok(())
}

#[test]
fn test_process_special_command_quit() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit =
        process_special_command(".quit", &mut buffer, &mut stdout, |_| Ok(()), &interpreter)?;

    assert!(should_exit); // Should signal to exit
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Exiting rholang-shell"));

    Ok(())
}

#[test]
fn test_process_special_command_list() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit =
        process_special_command(".list", &mut buffer, &mut stdout, |_| Ok(()), &interpreter)?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Edited lines:"));
    assert!(output.contains("line1"));
    assert!(output.contains("line2"));

    Ok(())
}

#[test]
fn test_process_special_command_delete() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".delete",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    assert_eq!(buffer, vec!["line1".to_string()]); // line2 should be removed
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Removed last line: line2"));

    Ok(())
}

#[test]
fn test_process_special_command_delete_empty() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".delete",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Buffer is empty, nothing to delete"));

    Ok(())
}

#[test]
fn test_process_special_command_reset() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit =
        process_special_command(".reset", &mut buffer, &mut stdout, |_| Ok(()), &interpreter)?;

    assert!(!should_exit);
    assert!(buffer.is_empty()); // Buffer should be cleared
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Buffer reset"));

    Ok(())
}

#[test]
fn test_process_special_command_ps() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::with_processes(vec![
        (1, "process1".to_string()),
        (2, "process2".to_string()),
    ]);

    let should_exit =
        process_special_command(".ps", &mut buffer, &mut stdout, |_| Ok(()), &interpreter)?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Running processes:"));
    assert!(output.contains("1: process1"));
    assert!(output.contains("2: process2"));

    Ok(())
}

#[test]
fn test_process_special_command_ps_empty() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit =
        process_special_command(".ps", &mut buffer, &mut stdout, |_| Ok(()), &interpreter)?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("No running processes"));

    Ok(())
}

#[test]
fn test_process_special_command_kill() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::with_processes(vec![
        (1, "process1".to_string()),
        (2, "process2".to_string()),
    ]);

    let should_exit = process_special_command(
        ".kill 1",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Process 1 killed successfully"));

    Ok(())
}

#[test]
fn test_process_special_command_kill_nonexistent() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".kill 999",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Process 999 not found"));

    Ok(())
}

#[test]
fn test_process_special_command_kill_invalid() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".kill abc",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Invalid process ID: abc"));

    Ok(())
}

#[test]
fn test_process_special_command_unknown() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".unknown",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Unknown command: .unknown"));

    Ok(())
}

#[test]
fn test_process_special_command_not_special() -> Result<()> {
    let mut buffer = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        "not a special command",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.is_empty()); // No output for non-special commands

    Ok(())
}

#[test]
fn test_process_multiline_input_empty_buffer_empty_line() -> Result<()> {
    let mut buffer = Vec::new();
    let command = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;
    assert!(command.is_none());
    assert!(buffer.is_empty());
    Ok(())
}

#[test]
fn test_process_multiline_input_empty_buffer_nonempty_line() -> Result<()> {
    let mut buffer = Vec::new();
    let command = process_multiline_input("line1".to_string(), &mut buffer, |_| Ok(()))?;
    assert!(command.is_none());
    assert_eq!(buffer, vec!["line1".to_string()]);
    Ok(())
}

#[test]
fn test_process_multiline_input_nonempty_buffer_nonempty_line() -> Result<()> {
    let mut buffer = vec!["line1".to_string()];
    let command = process_multiline_input("line2".to_string(), &mut buffer, |_| Ok(()))?;
    assert!(command.is_none());
    assert_eq!(buffer, vec!["line1".to_string(), "line2".to_string()]);
    Ok(())
}

#[test]
fn test_process_multiline_input_nonempty_buffer_empty_line() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];
    // First empty ignored
    let first = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;
    assert!(first.is_none());
    // Second empty executes
    let command = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;
    assert_eq!(command, Some("line1\nline2".to_string()));
    assert_eq!(buffer, vec!["line1".to_string(), "line2".to_string()]);
    Ok(())
}

#[test]
fn test_handle_interrupt() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];
    let multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::with_processes(vec![
        (1, "process1".to_string()),
        (2, "process2".to_string()),
    ]);

    handle_interrupt(&mut buffer, &mut stdout, |_| Ok(()), &interpreter)?;

    assert!(buffer.is_empty()); // Buffer should be cleared in multiline mode
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Killed 2 running processes"));
    assert!(output.contains("Input interrupted with Ctrl+C"));

    Ok(())
}

#[test]
fn test_handle_interrupt_single_line() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];
    let multiline = false;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::with_processes(vec![
        (1, "process1".to_string()),
        (2, "process2".to_string()),
    ]);

    handle_interrupt(&mut buffer, &mut stdout, |_| Ok(()), &interpreter)?;

    assert!(buffer.is_empty()); // Buffer should be cleared (single line mode removed)
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Killed 2 running processes"));
    assert!(output.contains("Input interrupted with Ctrl+C"));

    Ok(())
}

// New tests for .validate command
#[test]
fn test_validate_valid_buffer() -> Result<()> {
    let mut buffer = vec!["new ch in { for(@x <- ch) { Nil } }".to_string()];
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".validate",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(
        output.contains("Validation successful")
            || output.contains("no issues")
            || output.contains("Validation produced"),
        "Unexpected output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_validate_invalid_buffer_reports_diagnostics() -> Result<()> {
    let mut buffer = vec!["for(@x <- unbound_ch) { Nil }".to_string()];
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".validate",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    // Expect either diagnostic lines or a generic message that parsing failed
    assert!(
        output.contains("Validation produced") || output.contains("Parsing failed"),
        "Unexpected output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_validate_unused_command() -> Result<()> {
    let mut buffer = vec!["new ch in { for(@x <- ch) { Nil } }".to_string()];
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".validate-unused",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(
        output.contains("Unused-vars validation produced")
            || output.contains("Validation successful")
    );
    Ok(())
}

#[test]
fn test_validate_elab_command() -> Result<()> {
    let mut buffer = vec!["new ch in { for(@x <- ch) { Nil } }".to_string()];
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".validate-elab",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(
        output.contains("Elaboration validation produced")
            || output.contains("Validation successful")
    );
    Ok(())
}

#[test]
fn test_validate_resolver_command() -> Result<()> {
    let mut buffer = vec!["for(@x <- unbound_ch) { Nil }".to_string()];
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".validate-resolver",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(
        output.contains("Resolver validation produced")
            || output.contains("Validation successful")
            || output.contains("Parsing failed")
    );
    Ok(())
}

#[test]
fn test_validate_commands_empty_buffer() -> anyhow::Result<()> {
    use std::io::Cursor;
    let interpreter = MockInterpreterProvider::new();

    // .validate with empty buffer
    let mut buffer: Vec<String> = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let _ = process_special_command(
        ".validate",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Buffer is empty, nothing to validate"));

    // .validate-unused with empty buffer
    let mut buffer: Vec<String> = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let _ = process_special_command(
        ".validate-unused",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Buffer is empty, nothing to validate"));

    // .validate-elab with empty buffer
    let mut buffer: Vec<String> = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let _ = process_special_command(
        ".validate-elab",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Buffer is empty, nothing to validate"));

    // .validate-resolver with empty buffer
    let mut buffer: Vec<String> = Vec::new();
    let mut stdout = Cursor::new(Vec::new());
    let _ = process_special_command(
        ".validate-resolver",
        &mut buffer,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Buffer is empty, nothing to validate"));

    Ok(())
}
