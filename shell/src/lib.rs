pub mod providers;

use anyhow::Result;
use clap::Parser;
use std::io::Write;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Enable multiline mode
    #[arg(short, long, default_value_t = false)]
    pub multiline: bool,
}

pub fn help_message() -> String {
    "Available commands:".to_string()
        + "\n  .help, - Show this help message"
        + "\n  .mode - Toggle between multiline and single line modes"
        + "\n  .list - List all edited lines"
        + "\n  .delete or .del - Remove the last edited line"
        + "\n  .reset or Ctrl+C - Interrupt current input (in multiline mode: clear buffer)"
        + "\n  .quit - Exit the shell"
}

/// Process a special command (starting with '.')
/// Returns true if the command was processed, false otherwise
pub fn process_special_command<W: Write>(
    command: &str,
    buffer: &mut Vec<String>,
    multiline: &mut bool,
    stdout: &mut W,
    update_prompt: impl FnOnce(&str) -> Result<()>,
) -> Result<bool> {
    if !command.starts_with('.') {
        return Ok(false);
    }

    match command {
        ".help" => {
            writeln!(stdout, "{}", help_message())?;
        }
        ".mode" => {
            // Toggle multiline mode
            *multiline = !*multiline;
            let mode_message = if *multiline {
                "Switched to multiline mode (enter twice to execute)"
            } else {
                buffer.clear();
                update_prompt(">>> ")?;
                "Switched to single line mode"
            };
            writeln!(stdout, "{mode_message}")?;
        }
        ".quit" => {
            writeln!(stdout, "Exiting shell...")?;
            return Ok(true); // Signal to exit
        }
        ".list" => {
            writeln!(stdout, "Edited lines:")?;
            for line in buffer.iter() {
                writeln!(stdout, "{line}")?;
            }
        }
        ".delete" | ".del" => {
            if !buffer.is_empty() {
                let removed = buffer.pop().unwrap();
                writeln!(stdout, "Removed last line: {removed}")?;
            } else {
                writeln!(stdout, "Buffer is empty, nothing to delete")?;
            }
        }
        ".reset" => {
            buffer.clear();
            update_prompt(">>> ")?;
            writeln!(stdout, "Buffer reset")?;
        }
        ".buffer" => {
            writeln!(stdout, "Current buffer: {:?}", buffer)?;
        }
        _ => {
            writeln!(stdout, "Unknown command: {command}")?;
        }
    }
    Ok(false) // Don't exit
}

/// Process a line of input in multiline mode
/// Returns Some(command) if a command is ready to be executed, None otherwise
pub fn process_multiline_input(
    line: String,
    buffer: &mut Vec<String>,
    update_prompt: impl FnOnce(&str) -> Result<()>,
) -> Result<Option<String>> {
    if buffer.is_empty() {
        if line.is_empty() {
            return Ok(None);
        }
        *buffer = vec![line];
        update_prompt("... ")?;
        return Ok(None);
    }

    if !line.is_empty() {
        buffer.push(line);
        return Ok(None);
    }

    let command = buffer.join("\n");
    buffer.clear();
    update_prompt(">>> ")?;
    Ok(Some(command))
}

/// Process a line of input in single line mode
/// Returns Some(command) if a command is ready to be executed, None otherwise
pub fn process_single_line_input(line: String) -> Option<String> {
    if line.is_empty() {
        return None;
    }
    Some(line)
}

/// Handle an interrupt event (Ctrl+C)
pub fn handle_interrupt<W: Write>(
    buffer: &mut Vec<String>,
    multiline: bool,
    stdout: &mut W,
    update_prompt: impl FnOnce(&str) -> Result<()>,
) -> Result<()> {
    // Clear buffer in multiline mode
    if multiline {
        buffer.clear();
        update_prompt(">>> ")?;
    }

    writeln!(stdout, "Input interrupted with Ctrl+C")?;
    Ok(())
}
