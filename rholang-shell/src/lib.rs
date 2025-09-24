pub mod providers;

use anyhow::Result;
use bracket_parser::{BracketParser, BracketState};
use clap::Parser;
use providers::{InterpretationResult, InterpreterProvider};
use rustyline_async::{Readline, ReadlineEvent};
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
        + "\n  .ps - List all running processes"
        + "\n  .kill <index> - Kill a running process by index"
        + "\n  .quit - Exit the rholang-shell"
}

const DEFAULT_PROMPT: &str = ">>> ";

fn handle_kill_command<W: Write, I: InterpreterProvider>(
    arg: &str,
    stdout: &mut W,
    interpreter: &I,
) -> Result<()> {
    let pid_str = arg.trim();
    if pid_str.is_empty() {
        writeln!(stdout, "Usage: .kill <pid>")?;
        return Ok(());
    }
    match pid_str.parse::<usize>() {
        Ok(pid) => match interpreter.kill_process(pid) {
            Ok(true) => writeln!(stdout, "Process {} killed successfully", pid)?,
            Ok(false) => writeln!(stdout, "Process {} not found", pid)?,
            Err(e) => writeln!(stdout, "Error killing process {}: {}", pid, e)?,
        },
        Err(_) => writeln!(stdout, "Invalid process ID: {}", pid_str)?,
    }
    Ok(())
}

fn print_processes<W: Write, I: InterpreterProvider>(
    stdout: &mut W,
    interpreter: &I,
) -> Result<()> {
    match interpreter.list_processes() {
        Ok(processes) => {
            if processes.is_empty() {
                writeln!(stdout, "No running processes")?;
            } else {
                writeln!(stdout, "Running processes:")?;
                for (pid, code) in processes {
                    writeln!(stdout, "  {}: {}", pid, code)?;
                }
            }
        }
        Err(e) => writeln!(stdout, "Error listing processes: {}", e)?,
    }
    Ok(())
}

/// Process a special command (starting with '.')
/// Returns true if the command was processed, false otherwise
pub fn process_special_command<W: Write, I: InterpreterProvider>(
    command: &str,
    buffer: &mut Vec<String>,
    multiline: &mut bool,
    stdout: &mut W,
    update_prompt: impl FnOnce(&str) -> Result<()>,
    interpreter: &I,
) -> Result<bool> {
    let trimmed = command.trim();
    if !trimmed.starts_with('.') {
        return Ok(false);
    }

    let (cmd, arg) = trimmed.split_once(' ').map_or((trimmed, ""), |(c, a)| (c, a.trim()));

    match cmd {
        ".help" => {
            writeln!(stdout, "{}", help_message())?;
        }
        ".mode" => {
            // Toggle multiline mode
            *multiline = !*multiline;
            let mode_msg = if *multiline {
                "Switched to multiline mode (enter twice to execute)"
            } else {
                buffer.clear();
                update_prompt(DEFAULT_PROMPT)?;
                "Switched to single line mode"
            };
            writeln!(stdout, "{mode_msg}")?;
        }
        ".quit" => {
            writeln!(stdout, "Exiting rholang-shell...")?;
            return Ok(true); // Signal to exit
        }
        ".list" => {
            writeln!(stdout, "Edited lines:")?;
            for line in buffer.iter() {
                writeln!(stdout, "{line}")?;
            }
        }
        ".delete" | ".del" => {
            if let Some(removed) = buffer.pop() {
                writeln!(stdout, "Removed last line: {removed}")?;
            } else {
                writeln!(stdout, "Buffer is empty, nothing to delete")?;
            }
        }
        ".reset" => {
            buffer.clear();
            update_prompt(DEFAULT_PROMPT)?;
            writeln!(stdout, "Buffer reset")?;
        }
        ".buffer" => {
            writeln!(stdout, "Current buffer: {:?}", buffer)?;
        }
        ".ps" => {
            print_processes(stdout, interpreter)?;
        }
        ".kill" => {
            handle_kill_command(arg, stdout, interpreter)?;
        }
        _ => {
            writeln!(stdout, "Unknown command: {command}")?;
        }
    }
    Ok(false) // Don't exit
}

// ... existing code ...

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
/// If the line ends inside brackets, switches to multiline mode and returns None
pub fn process_single_line_input(
    line: String,
    buffer: &mut Vec<String>,
    multiline: &mut bool,
    update_prompt: impl FnOnce(&str) -> Result<()>,
) -> Result<Option<String>> {
    if line.is_empty() {
        return Ok(None);
    }

    // Check if the line ends inside brackets
    let mut bracket_parser = match BracketParser::new() {
        Ok(parser) => parser,
        Err(_e) => {
            // If we can't create the parser, just execute the line normally
            // This is a fallback in case of an error
            return Ok(Some(line));
        }
    };

    let state = bracket_parser.get_final_state(&line);

    if state == BracketState::Inside {
        // Line ends inside brackets, switch to multiline mode
        *multiline = true;
        buffer.push(line);
        update_prompt("... ")?;
        return Ok(None);
    }

    // Line doesn't end inside brackets, execute it immediately
    Ok(Some(line))
}

/// Handle an interrupt event (Ctrl+C)
pub fn handle_interrupt<W: Write, I: InterpreterProvider>(
    buffer: &mut Vec<String>,
    multiline: bool,
    stdout: &mut W,
    update_prompt: impl FnOnce(&str) -> Result<()>,
    interpreter: &I,
) -> Result<()> {
    // Clear buffer in multiline mode
    if multiline {
        buffer.clear();
        update_prompt(">>> ")?;
    }

    // Kill all running processes
    match interpreter.kill_all_processes() {
        Ok(count) => {
            if count > 0 {
                writeln!(stdout, "Killed {} running processes", count)?;
            }
        }
        Err(e) => writeln!(stdout, "Error killing processes: {}", e)?,
    }

    writeln!(stdout, "Input interrupted with Ctrl+C")?;
    Ok(())
}

/// Run the rholang-shell with the provided interpreter provider
pub async fn run_shell<I: InterpreterProvider>(args: Args, interpreter: I) -> Result<()> {
    // If stdin is not a TTY, run in non-interactive (batch) mode and read from stdin
    if !atty::is(atty::Stream::Stdin) {
        use std::io::{self, Read};
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        let input = input.trim().to_string();
        if input.is_empty() {
            return Ok(());
        }
        let result = interpreter.interpret(&input).await;
        match result {
            InterpretationResult::Success(output) => {
                println!("{}", output);
            }
            InterpretationResult::Error(e) => {
                eprintln!("Error: {}", e);
                // Non-zero exit if error in batch mode
                // But since function returns Result, propagate as Ok to avoid panics for now
            }
        }
        return Ok(());
    }

    writeln!(std::io::stdout(), "{}", help_message())?;

    let prompt = ">>> ".to_string();

    let (mut rl, mut stdout) = Readline::new(prompt.clone())?;
    let mut buffer: Vec<String> = Vec::new();
    let mut multiline = args.multiline;

    rl.should_print_line_on(true, false);

    loop {
        tokio::select! {
            cmd = rl.readline() => match cmd {
                Ok(ReadlineEvent::Line(line)) => {
                    let line = line.trim().to_string();

                    // Process special commands
                    let should_exit = process_special_command(
                        &line,
                        &mut buffer,
                        &mut multiline,
                        &mut stdout,
                        |prompt| Ok(rl.update_prompt(prompt)?),
                        &interpreter,
                    )?;

                    if should_exit {
                        break;
                    }

                    if line.starts_with('.') {
                        continue;
                    }

                    rl.add_history_entry(line.clone());

                    // Process input based on mode
                    let command_option = if multiline {
                        process_multiline_input(
                            line,
                            &mut buffer,
                            |prompt| Ok(rl.update_prompt(prompt)?),
                        )?
                    } else {
                        process_single_line_input(
                            line,
                            &mut buffer,
                            &mut multiline,
                            |prompt| Ok(rl.update_prompt(prompt)?),
                        )?
                    };

                    // Execute command if one is ready
                    if let Some(command) = command_option {
                        writeln!(stdout, "Executing code: {command}")?;
                        let result = interpreter.interpret(&command).await;
                        match result {
                            InterpretationResult::Success(output) => writeln!(stdout, "Output: {output}")?,
                            InterpretationResult::Error(e) => writeln!(stdout, "Error interpreting line: {e}")?,
                        }
                    }
                }
                Ok(ReadlineEvent::Eof) => {
                    break;
                }
                Ok(ReadlineEvent::Interrupted) => {
                    handle_interrupt(
                        &mut buffer,
                        multiline,
                        &mut stdout,
                        |prompt| Ok(rl.update_prompt(prompt)?),
                        &interpreter,
                    )?;
                    continue;
                }
                Err(e) => {
                    writeln!(stdout, "Error: {e:?}")?;
                    break;
                }
            }
        }
    }
    rl.flush()?;
    Ok(())
}
