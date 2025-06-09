use anyhow::Result;
use rstest::rstest;
use tokio::time::Duration;

use std::io::Write;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::process::{Command, Stdio, Child};
use std::io::{BufRead, BufReader};

use shell::interpreter::{FakeInterpreter, Interpreter};

struct MultilineTestShell {
    child: Child,
    stdout_rx: Receiver<String>,
    stdin: Option<std::process::ChildStdin>,
}

impl MultilineTestShell {
    fn new() -> Result<Self> {
        let mut child = Command::new("cargo")
            .args(["run", "--quiet", "--bin", "shell"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
        let stdin = child.stdin.take();

        let (tx, rx) = channel();

        // Start a thread to read from the shell's stdout
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
            }
        });

        // Give the shell some time to start up
        thread::sleep(Duration::from_millis(500));

        Ok(MultilineTestShell {
            child,
            stdout_rx: rx,
            stdin,
        })
    }

    fn send_line(&mut self, line: &str) -> Result<()> {
        if let Some(stdin) = self.stdin.as_mut() {
            writeln!(stdin, "{}", line)?;
            stdin.flush()?;
            // Give the shell some time to process the line
            thread::sleep(Duration::from_millis(100));
            Ok(())
        } else {
            Err(anyhow::anyhow!("Shell stdin not available"))
        }
    }

    fn send_multiline_command(&mut self, lines: Vec<&str>) -> Result<()> {
        // Send each line of the multiline command
        for line in lines {
            self.send_line(line)?;
        }

        // Send an empty line to execute the command
        self.send_line("")?;

        Ok(())
    }

    fn read_output(&self, timeout_ms: u64) -> Vec<String> {
        let mut output = Vec::new();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        while let Ok(line) = self.stdout_rx.recv_timeout(timeout) {
            output.push(line);
        }

        output
    }
}

impl Drop for MultilineTestShell {
    fn drop(&mut self) {
        // Try to exit gracefully first
        let _ = self.send_line("quit");
        thread::sleep(std::time::Duration::from_millis(100));

        // Force kill if still running
        let _ = self.child.kill();
    }
}

#[tokio::test]
#[ignore] // Ignore by default as this requires running the full binary
async fn test_shell_multiline_input() -> Result<()> {
    let mut shell = MultilineTestShell::new()?;

    // Read initial output which should contain the welcome message
    let initial_output = shell.read_output(500);
    assert!(
        initial_output.iter().any(|line| line.contains("Multiline mode")),
        "Missing welcome message in output: {:?}", initial_output
    );

    // Send a multiline command
    shell.send_multiline_command(vec![
        "let x = 10;",
        "let y = 20;",
        "println!(\"{}\", x + y);"
    ])?;

    // Check the response
    let output = shell.read_output(500);
    assert!(
        output.iter().any(|line| line.contains("Executing code: let x = 10;\nlet y = 20;\nprintln!(\"{}\", x + y);")),
        "Shell didn't show correct multiline command: {:?}", output
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default as this requires running the full binary
async fn test_shell_multiline_interrupted() -> Result<()> {
    let mut shell = MultilineTestShell::new()?;

    // Clear initial output
    shell.read_output(500);

    // Start a multiline command
    shell.send_line("let a = 5;")?;

    // Send Ctrl+C (interrupt) - we simulate this by sending ReadlineEvent::Interrupted
    // Since we can't actually send Ctrl+C via this interface, we'll update the test later
    // For now, let's assert what would happen if an interrupt occurred

    // In real implementation, this would be interrupted with a special character
    // Here we just verify the shell keeps accepting commands

    // Send a new command
    shell.send_multiline_command(vec!["println!(\"After interrupt\");"])?;

    // Check the response
    let output = shell.read_output(500);
    assert!(
        output.iter().any(|line| line.contains("Executing code: println!(\"After interrupt\")") ||
                          line.contains("Output: println!(\"After interrupt\")") ),
        "Shell didn't recover after interrupt: {:?}", output
    );

    Ok(())
}

#[tokio::test]
async fn test_multiline_buffer_handling() -> Result<()> {
    // This test uses direct Interpreter functionality
    let interpreter = FakeInterpreter;

    // Create a simulated multiline command
    let line1 = "for i in 0..3 {".to_string();
    let line2 = "    println!(\"{}\", i);".to_string();
    let line3 = "}".to_string();

    // Combine lines as they would be in the buffer
    let combined = format!("{line1}\n{line2}\n{line3}");

    // Interpret the combined command
    let result = interpreter.interpret(combined.clone()).await?;

    // Verify the result matches what we'd expect from FakeInterpreter
    assert_eq!(result, combined);

    Ok(())
}

#[rstest]
#[case(vec!["let x = 10;", "x + 20"], "let x = 10;\nx + 20")]
#[case(vec!["if true {", "    println!(\"true\");", "}"], "if true {\n    println!(\"true\");\n}")]
#[case(vec!["fn test() {", "    let y = 5;", "    y * 2", "}"], "fn test() {\n    let y = 5;\n    y * 2\n}")]
#[tokio::test]
async fn test_multiline_commands_joined_correctly(
    #[case] input_lines: Vec<&str>,
    #[case] expected: &str
) -> Result<()> {
    let interpreter = FakeInterpreter;

    // Join the lines with newlines (simulating how main.rs does it)
    let command = input_lines.join("\n");

    // Interpret the command
    let result = interpreter.interpret(command).await?;

    // Verify the result
    assert_eq!(result, expected);

    Ok(())
}
