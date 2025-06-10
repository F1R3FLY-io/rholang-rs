pub mod providers;

use clap::Parser;
use rustyline_async::{Readline, ReadlineEvent};
use std::io::Write;

use providers::{FakeInterpreterProvider, InterpreterProvider};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable multiline mode
    #[arg(short, long, default_value_t = true)]
    multiline: bool,
}

fn help_message() -> String {
    "Available commands:" .to_string() +
    "\n  .help, - Show this help message" +
    "\n  .mode - Toggle between multiline and single line modes" +
    "\n  .list - List all edited lines" +
    "\n  .delete or .del - Remove the last edited line" +
    "\n  .reset or Ctrl+C - Interrupt current input (in multiline mode: clear buffer)" +
    "\n  .quit - Exit the shell"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    writeln!(
        std::io::stdout(),
        "{}", help_message()
    )?;
    
    let prompt = ">>> ".to_string();

    let (mut rl, mut stdout) = Readline::new(prompt.clone())?;
    let interpreter = FakeInterpreterProvider;
    let mut buffer: Vec<String> = Vec::new();
    let mut multiline = args.multiline;


    rl.should_print_line_on(true, false);

    loop {
        tokio::select! {

            cmd = rl.readline() => match cmd {
                Ok(ReadlineEvent::Line(line)) => {
                    let line = line.trim().to_string();
                    
                    if line.starts_with('.'){
                        // Handle special commands starting with '.'
                        if line == ".help" {
                            writeln!(stdout, "{}", help_message())?;
                        } 
                        else if line == ".mode" {
                            // Toggle multiline mode
                            multiline = !multiline;
                            let mode_message = if multiline {
                                "Switched to multiline mode (enter twice to execute)"
                            } else {
                                buffer.clear();
                                rl.update_prompt(">>> ").expect("Can't update prompt");
                                "Switched to single line mode"
                            };
                            writeln!(stdout, "{mode_message}")?;
                        } 
                        else if line == ".quit" {
                            writeln!(stdout, "Exiting shell...")?;
                            break;
                        }
                        else if line == ".list" {
                            writeln!(stdout, "Edited lines:")?;
                            for line in buffer.clone().iter(){
                                writeln!(stdout, "{line}")?;
                            }
                        }
                        else if line == ".delete" || line == ".del" {
                            if !buffer.is_empty() {
                                let removed = buffer.pop().unwrap();
                                writeln!(stdout, "Removed last line: {removed}")?;
                            } else {
                                writeln!(stdout, "Buffer is empty, nothing to delete")?;
                            }
                        }
                        else if line == ".reset" {
                            buffer.clear();
                            rl.update_prompt(">>> ").expect("Can't update prompt");
                            writeln!(stdout, "Buffer reset")?;
                        }
                        else if line == ".buffer" {
                            writeln!(stdout, "Current buffer: {:?}", buffer)?;
                        } 
                        else {
                            writeln!(stdout, "Unknown command: {line}")?;
                        }
                        continue;
                    }
                    
                    rl.add_history_entry(line.clone());

                    let command  = if multiline {
                        if buffer.is_empty() {
                            if line.is_empty() {
                                continue;
                            }
                            buffer = vec![line];
                            rl.update_prompt("... ").expect("Can't update prompt");
                            continue;
                        }

                        if !line.is_empty() {
                            buffer.push(line);
                            continue;
                        }
                        let command = buffer.join("\n");
                        buffer.clear();
                        rl.update_prompt(">>> ").expect("Can't update prompt");
                        command
                    } else {
                        if line.is_empty() {
                            continue;
                        }
                        line
                    };

                    writeln!(stdout, "Executing code: {command}")?;
                    let result = interpreter.interpret(command).await;
                    match result {
                        Ok(output) => writeln!(stdout, "Output: {output}")?,
                        Err(e) => writeln!(stdout, "Error interpreting line: {e}")?,
                    }
                }
                Ok(ReadlineEvent::Eof) => {
                    break;
                }
                Ok(ReadlineEvent::Interrupted) => {
                    // Clear buffer in multiline mode
                    if multiline {
                        buffer.clear();
                        rl.update_prompt(">>> ").expect("Can't update prompt");
                    }

                    writeln!(stdout, "Input interrupted with Ctrl+C")?;
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
