use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};

struct TestShell {
    child: Child,
    stdout_rx: Receiver<String>,
    stdin: Option<std::process::ChildStdin>,
}

impl TestShell {
    fn new() -> Result<Self> {
        let mut child = Command::new("cargo")
            .args(["run", "--quiet", "--bin", "shell"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to capture stdout"))?;
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

        Ok(TestShell {
            child,
            stdout_rx: rx,
            stdin,
        })
    }

    fn send_command(&mut self, command: &str) -> Result<()> {
        if let Some(stdin) = self.stdin.as_mut() {
            writeln!(stdin, "{}", command)?;
            stdin.flush()?;
            // Give the shell some time to process the command
            thread::sleep(Duration::from_millis(100));
            Ok(())
        } else {
            Err(anyhow!("Shell stdin not available"))
        }
    }

    fn read_output(&self, timeout_ms: u64) -> Vec<String> {
        let mut output = Vec::new();
        let timeout = Duration::from_millis(timeout_ms);

        while let Ok(line) = self.stdout_rx.recv_timeout(timeout) {
            output.push(line);
        }

        output
    }
}

impl Drop for TestShell {
    fn drop(&mut self) {
        // Try to exit gracefully first
        let _ = self.send_command("quit");
        thread::sleep(Duration::from_millis(100));

        // Force kill if still running
        let _ = self.child.kill();
    }
}

#[test]
#[ignore] // Ignore by default as this requires running the full binary
fn test_shell_basic_interaction() -> Result<()> {
    let mut shell = TestShell::new()?;

    // Check for the prompt in initial output
    let initial_output = shell.read_output(500);
    assert!(initial_output.is_empty(), "Unexpected initial output: {:?}", initial_output);

    // Send a simple command
    shell.send_command("echo test")?;

    // Check the response
    let output = shell.read_output(500);
    assert!(
        output.iter().any(|line| line.contains("You entered: \"echo test\"")),
        "Shell didn't echo our command: {:?}", output
    );

    // Exit the shell
    shell.send_command("quit")?;

    Ok(())
}
