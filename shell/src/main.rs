mod main_sync;
pub mod interpreter;
pub mod multiline_helper;

use rustyline_async::{Readline, ReadlineEvent};
use std::io::Write;

use interpreter::{FakeInterpreter, Interpreter};
use multiline_helper::process_line;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prompt = ">>> ";
    let (mut rl, mut stdout) = Readline::new(prompt.into())?;
    let interpreter = FakeInterpreter;
    let mut buffer = String::new();
    let mut in_multiline_mode = false;

    rl.should_print_line_on(false, false);

    loop {
        // Update prompt if needed
        let current_prompt = if in_multiline_mode { "... " } else { ">>> " };
        if prompt != current_prompt {
            prompt = current_prompt;
            // Recreate readline with new prompt
            (rl, stdout) = Readline::new(prompt.into())?;
            rl.should_print_line_on(false, false);
        }

        tokio::select! {
            cmd = rl.readline() => match cmd {
                Ok(ReadlineEvent::Line(line)) => {
                    // Add to history only complete commands
                    if !in_multiline_mode {
                        rl.add_history_entry(line.clone());
                    }

                    // Handle special commands
                    if !in_multiline_mode && line == "quit" {
                        writeln!(stdout, "Goodbye!")?;
                        break;
                    }

                    // Process the line and check if we're ready to execute
                    let ready_to_execute = process_line(&line, &mut buffer, &mut in_multiline_mode);

                    // Show appropriate messages based on state
                    if !in_multiline_mode && line.is_empty() {
                        writeln!(stdout, "Entering multiline mode. Type an empty line twice to execute.")?;
                    } else if ready_to_execute {
                        // Execute the command
                        writeln!(stdout, "Executing code:")?;
                        writeln!(stdout, "---------------------")?;
                        writeln!(stdout, "{}", buffer)?;
                        writeln!(stdout, "---------------------")?;

                        // Send to interpreter
                        let result = interpreter.interpret(buffer.clone()).await;
                        match result {
                            Ok(output) => writeln!(stdout, "Output: {output}")?,
                            Err(e) => writeln!(stdout, "Error interpreting code: {e}")?,
                        }

                        // Reset buffer
                        buffer.clear();
                    }
                }
                Ok(ReadlineEvent::Eof) => {
                    writeln!(stdout, "<EOF>")?;
                    break;
                }
                Ok(ReadlineEvent::Interrupted) => {
                    // Clear the buffer and exit multiline mode if we're interrupted
                    if in_multiline_mode || !buffer.is_empty() {
                        writeln!(stdout, "^C (Cleared buffer)")?;
                        buffer.clear();
                        in_multiline_mode = false;
                    } else {
                        continue;
                    }
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
