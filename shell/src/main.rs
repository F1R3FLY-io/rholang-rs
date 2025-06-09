pub mod interpreter;
mod main_sync;

use clap::Parser;
use rustyline_async::{Readline, ReadlineEvent};
use std::io::Write;

use interpreter::{FakeInterpreter, Interpreter};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable multiline mode
    #[arg(short, long, default_value_t = true)]
    multiline: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.multiline {
        writeln!(
            std::io::stdout(),
            "Multiline mode, to call command press enter twice."
        )?;
    } else {
        writeln!(
            std::io::stdout(),
            "Single line mode, each command executes immediately."
        )?;
    }

    writeln!(
        std::io::stdout(),
        "Available commands:\n  .mode or \\mode - Toggle between multiline/single-line modes\n  .help or \\help - Show all available commands\n"
    )?;

    let (mut rl, mut stdout) = Readline::new(">>> ".to_string())?;
    let interpreter = FakeInterpreter;
    let mut buffer: Vec<String> = Vec::new();
    let mut multiline = args.multiline;

    rl.should_print_line_on(true, false);

    loop {
        tokio::select! {
            // _ = sleep(Duration::from_secs(1)) => {
            // 	writeln!(stdout, "Message received!")?;
            // }

            cmd = rl.readline() => match cmd {
                Ok(ReadlineEvent::Line(line)) => {
                    let line = line.trim().to_string();

                    let command  = if (multiline){
                        if buffer.is_empty() {
                            if(line.is_empty()) {
                                continue;
                            }
                            rl.add_history_entry(line.clone());
                            buffer = vec![line];
                            rl.update_prompt("... ").expect("Can't update prompt");
                            continue;
                        }

                        if(!line.is_empty()) {
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
                        rl.add_history_entry(line.clone());
                        line
                    };
                    
                    // Special commands
                    if command == "quit" || command == "exit" {
                        break;
                    } else if command == ".mode" || command == "\\mode" {
                        // Toggle multiline mode
                        multiline = !multiline;
                        let mode_message = if multiline {
                            "Switched to multiline mode (enter twice to execute)"
                        } else {
                            "Switched to single line mode"
                        };
                        writeln!(stdout, "{mode_message}")?;
                        continue;
                    } else if command == ".help" || command == "\\help" || command == "help" {
                        writeln!(stdout, "Available commands:")?;
                        writeln!(stdout, "  .help, \\help, help - Show this help message")?;
                        writeln!(stdout, "  .mode, \\mode - Toggle between multiline and single line modes")?;
                        writeln!(stdout, "  Ctrl+C - Interrupt current input (in multiline mode: clear buffer)")?;
                        writeln!(stdout, "  quit, exit - Exit the shell")?;
                        continue;
                    }

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
