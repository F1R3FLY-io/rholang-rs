mod main_sync;
mod interpreter;
// mod rh_interpreter;

use rustyline_async::{Readline, ReadlineEvent};
use std::io::Write;
use std::time::Duration;
use tokio::time::sleep;

use interpreter::{FakeInterpreter, Interpreter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let (mut rl, mut stdout) = Readline::new(">>> ".into())?;
	let interpreter = FakeInterpreter;

	rl.should_print_line_on(false, false);

	loop {
		tokio::select! {
			// _ = sleep(Duration::from_secs(1)) => {
			// 	writeln!(stdout, "Message received!")?;
			// }
			cmd = rl.readline() => match cmd {
				Ok(ReadlineEvent::Line(line)) => {
					writeln!(stdout, "You entered: {line:?}")?;
					rl.add_history_entry(line.clone());
					if line == "quit" {
						break;
					}
					let result = interpreter.interpret(line).await;
					match result {
						Ok(output) => writeln!(stdout, "Output: {output}")?,
						Err(e) => writeln!(stdout, "Error interpreting line: {e}")?,
					}
				}
				Ok(ReadlineEvent::Eof) => {
					writeln!(stdout, "<EOF>")?;
					break;
				}
				Ok(ReadlineEvent::Interrupted) => {
					// writeln!(stdout, "^C")?;
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
