mod main_sync;
pub mod interpreter;

use rustyline_async::{Readline, ReadlineEvent};
use std::io::Write;

use interpreter::{FakeInterpreter, Interpreter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	writeln!(std::io::stdout(), "Multiline mode, to call command press enter twice.\n")?;
    let (mut rl, mut stdout) = Readline::new(">>> ".to_string())?;
    let interpreter = FakeInterpreter;
    let mut buffer: Vec<String> = Vec::new();

    rl.should_print_line_on(true, false);

	loop {
		tokio::select! {
			// _ = sleep(Duration::from_secs(1)) => {
			// 	writeln!(stdout, "Message received!")?;
			// }

			cmd = rl.readline() => match cmd {
				Ok(ReadlineEvent::Line(line)) => {
					if buffer.is_empty() {
						let line = line.trim().to_string();
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

					// let history_buffer = buffer.clone().join(" ");
					let command = buffer.join("\n");
					buffer.clear();
					rl.update_prompt(">>> ").expect("Can't update prompt");

					if command == "quit" {
						break;
					}

					// rl.add_history_entry(history_buffer);

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
					writeln!(stdout, "Reset")?;
					buffer.clear();
					rl.update_prompt(">>> ").expect("Can't update prompt");
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
