use anyhow::Result;
use shell::{
    handle_interrupt, help_message, process_multiline_input, process_single_line_input,
    process_special_command, providers::InterpretationResult, Args,
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
impl shell::providers::InterpreterProvider for MockInterpreterProvider {
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
    assert!(message.contains(".mode"));
    assert!(message.contains(".list"));
    assert!(message.contains(".delete"));
    assert!(message.contains(".reset"));
    assert!(message.contains(".ps"));
    assert!(message.contains(".kill"));
    assert!(message.contains(".quit"));
}

#[test]
fn test_process_special_command_help() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = false;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".help",
        &mut buffer,
        &mut multiline,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains(".help"));
    assert!(output.contains(".mode"));

    Ok(())
}

#[test]
fn test_process_special_command_mode() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = false;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".mode",
        &mut buffer,
        &mut multiline,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    assert!(multiline); // Should have toggled to true
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Switched to multiline mode"));

    Ok(())
}

#[test]
fn test_process_special_command_quit() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = false;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".quit",
        &mut buffer,
        &mut multiline,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(should_exit); // Should signal to exit
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Exiting shell"));

    Ok(())
}

#[test]
fn test_process_special_command_list() -> Result<()> {
    let mut buffer = vec!["line1".to_string(), "line2".to_string()];
    let mut multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".list",
        &mut buffer,
        &mut multiline,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

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
    let mut multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".delete",
        &mut buffer,
        &mut multiline,
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
    let mut multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".delete",
        &mut buffer,
        &mut multiline,
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
    let mut multiline = true;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".reset",
        &mut buffer,
        &mut multiline,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    assert!(buffer.is_empty()); // Buffer should be cleared
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Buffer reset"));

    Ok(())
}

#[test]
fn test_process_special_command_ps() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = false;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::with_processes(vec![
        (1, "process1".to_string()),
        (2, "process2".to_string()),
    ]);

    let should_exit = process_special_command(
        ".ps",
        &mut buffer,
        &mut multiline,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

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
    let mut multiline = false;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".ps",
        &mut buffer,
        &mut multiline,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert!(!should_exit);
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("No running processes"));

    Ok(())
}

#[test]
fn test_process_special_command_kill() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = false;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::with_processes(vec![
        (1, "process1".to_string()),
        (2, "process2".to_string()),
    ]);

    let should_exit = process_special_command(
        ".kill 1",
        &mut buffer,
        &mut multiline,
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
    let mut multiline = false;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".kill 999",
        &mut buffer,
        &mut multiline,
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
    let mut multiline = false;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".kill abc",
        &mut buffer,
        &mut multiline,
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
    let mut multiline = false;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        ".unknown",
        &mut buffer,
        &mut multiline,
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
    let mut multiline = false;
    let mut stdout = Cursor::new(Vec::new());
    let interpreter = MockInterpreterProvider::new();

    let should_exit = process_special_command(
        "not a special command",
        &mut buffer,
        &mut multiline,
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
    let command = process_multiline_input("".to_string(), &mut buffer, |_| Ok(()))?;
    assert_eq!(command, Some("line1\nline2".to_string()));
    assert!(buffer.is_empty());
    Ok(())
}

#[test]
fn test_process_single_line_input_empty_line() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = false;
    let command =
        process_single_line_input("".to_string(), &mut buffer, &mut multiline, |_| Ok(()))?;
    assert!(command.is_none());
    assert!(buffer.is_empty());
    assert!(!multiline);
    Ok(())
}

#[test]
fn test_process_single_line_input_complete_line() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = false;
    let command =
        process_single_line_input("1 + 2".to_string(), &mut buffer, &mut multiline, |_| Ok(()))?;
    assert_eq!(command, Some("1 + 2".to_string()));
    assert!(buffer.is_empty());
    assert!(!multiline);
    Ok(())
}

#[test]
fn test_process_single_line_input_incomplete_line() -> Result<()> {
    let mut buffer = Vec::new();
    let mut multiline = false;
    let command = process_single_line_input(
        "new x in {".to_string(),
        &mut buffer,
        &mut multiline,
        |_| Ok(()),
    )?;
    assert!(command.is_none());
    assert_eq!(buffer, vec!["new x in {".to_string()]);
    assert!(multiline);
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

    handle_interrupt(
        &mut buffer,
        multiline,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

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

    handle_interrupt(
        &mut buffer,
        multiline,
        &mut stdout,
        |_| Ok(()),
        &interpreter,
    )?;

    assert_eq!(buffer, vec!["line1".to_string(), "line2".to_string()]); // Buffer should not be cleared in single line mode
    let output = String::from_utf8(stdout.into_inner())?;
    assert!(output.contains("Killed 2 running processes"));
    assert!(output.contains("Input interrupted with Ctrl+C"));

    Ok(())
}

#[test]
fn test_args() {
    let args = Args { multiline: true };
    assert!(args.multiline);

    let args = Args { multiline: false };
    assert!(!args.multiline);
}
