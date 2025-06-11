use anyhow::Result;
use clap::Parser;
use rustyline_async::{Readline, ReadlineEvent};
use std::io::Write;

use shell::providers::{FakeInterpreterProvider, InterpreterProvider};
use shell::{
    handle_interrupt, help_message, process_multiline_input, process_single_line_input,
    process_special_command, Args,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    writeln!(std::io::stdout(), "{}", help_message())?;

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

                    // Process special commands
                    let should_exit = process_special_command(
                        &line,
                        &mut buffer,
                        &mut multiline,
                        &mut stdout,
                        |prompt| Ok(rl.update_prompt(prompt)?),
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
                        process_single_line_input(line)
                    };

                    // Execute command if one is ready
                    if let Some(command) = command_option {
                        writeln!(stdout, "Executing code: {command}")?;
                        let result = interpreter.interpret(&command).await;
                        match result {
                            Ok(output) => writeln!(stdout, "Output: {output}")?,
                            Err(e) => writeln!(stdout, "Error interpreting line: {e}")?,
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
