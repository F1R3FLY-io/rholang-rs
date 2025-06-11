use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rholang_fake::{FakeRholangInterpreter, InterpretationResult, InterpreterError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task;
use tokio::time::timeout;

/// Trait for interpreter providers
/// This trait defines the interface for interpreters that can be used with the shell
#[async_trait]
pub trait InterpreterProvider {
    /// Interpret a string of code and return the result
    async fn interpret(&self, code: &str) -> InterpretationResult;

    /// List all running processes
    /// Returns a vector of tuples containing the process ID and the code being executed
    fn list_processes(&self) -> Result<Vec<(usize, String)>>;

    /// Kill a process by ID
    /// Returns true if the process was killed, false if it wasn't found
    fn kill_process(&self, pid: usize) -> Result<bool>;

    /// Kill all running processes
    /// Returns the number of processes that were killed
    fn kill_all_processes(&self) -> Result<usize>;
}

/// A fake interpreter provider that simply returns the input code
/// This is used for testing and as a placeholder
pub struct FakeInterpreterProvider;

#[async_trait]
impl InterpreterProvider for FakeInterpreterProvider {
    async fn interpret(&self, code: &str) -> InterpretationResult {
        // Fake implementation: just returns the input code
        InterpretationResult::Success(code.to_string())
    }

    /// List all running processes
    /// This is a fake implementation that always returns an empty list
    /// since FakeInterpreterProvider doesn't actually manage processes
    fn list_processes(&self) -> Result<Vec<(usize, String)>> {
        // Fake implementation: no processes to list
        Ok(Vec::new())
    }

    /// Kill a process by ID
    /// This is a fake implementation that always returns false
    /// since FakeInterpreterProvider doesn't actually manage processes
    fn kill_process(&self, _pid: usize) -> Result<bool> {
        // Fake implementation: no processes to kill
        Ok(false)
    }

    /// Kill all running processes
    /// This is a fake implementation that always returns 0
    /// since FakeInterpreterProvider doesn't actually manage processes
    fn kill_all_processes(&self) -> Result<usize> {
        // Fake implementation: no processes to kill
        Ok(0)
    }
}

/// Information about a running interpreter process
struct ProcessInfo {
    /// The code being interpreted
    code: String,
    /// The cancel sender to abort the process
    cancel_sender: Option<oneshot::Sender<()>>,
}

/// Provider for the fake Rholang interpreter
/// This implements the InterpreterProvider trait
pub struct RholangFakeInterpreterProvider {
    /// Map of process ID to process information
    processes: Arc<Mutex<HashMap<usize, ProcessInfo>>>,
    /// Next process ID to assign
    next_pid: Arc<Mutex<usize>>,
}

impl RholangFakeInterpreterProvider {
    /// Create a new instance of the Rholang fake interpreter provider
    pub fn new() -> Result<Self> {
        Ok(RholangFakeInterpreterProvider {
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_pid: Arc::new(Mutex::new(1)),
        })
    }
}

/// Implementation of the InterpreterProvider trait for the fake Rholang interpreter
#[async_trait]
impl InterpreterProvider for RholangFakeInterpreterProvider {
    async fn interpret(&self, code: &str) -> InterpretationResult {
        // Create a new interpreter for each call to avoid mutability issues
        let mut interpreter = match FakeRholangInterpreter::new() {
            Ok(interpreter) => interpreter,
            Err(e) => {
                return InterpretationResult::Error(InterpreterError::other_error(format!(
                    "Failed to create interpreter: {}",
                    e
                )))
            }
        };

        // Use the interpreter to parse and validate the code
        if !interpreter.is_valid(code) {
            return InterpretationResult::Error(InterpreterError::parsing_error(
                "Invalid Rholang code",
                None,
                Some(code.to_string()),
            ));
        }

        // Clone the code for the process info and for the task
        let code_clone = code.to_string();
        let code_for_task = code.to_string();

        // Clone the Arc<Mutex<>> for the task
        let processes = Arc::clone(&self.processes);
        let next_pid = Arc::clone(&self.next_pid);

        // Create a oneshot channel for cancellation
        let (cancel_sender, cancel_receiver) = oneshot::channel();

        // Get the next process ID
        let pid = {
            let mut next_pid = match next_pid.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    return InterpretationResult::Error(InterpreterError::other_error(format!(
                        "Failed to lock next_pid: {}",
                        e
                    )))
                }
            };
            let pid = *next_pid;
            *next_pid += 1;
            pid
        };

        // Store the process info
        {
            let mut processes = match processes.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    return InterpretationResult::Error(InterpreterError::other_error(format!(
                        "Failed to lock processes: {}",
                        e
                    )))
                }
            };
            processes.insert(
                pid,
                ProcessInfo {
                    code: code_clone,
                    cancel_sender: Some(cancel_sender),
                },
            );
        }

        // Spawn a task to run the interpreter asynchronously
        let handle = task::spawn(async move {
            // Create a future that completes when the cancel signal is received
            let cancel_future = cancel_receiver;

            // Create a future that completes when the interpreter finishes
            let interpret_future = interpreter.interpret_async(&code_for_task);

            // Run the interpreter with a timeout
            let timeout_future = timeout(Duration::from_secs(30), interpret_future);

            // Wait for either the interpreter to finish, the timeout to expire, or the cancel signal to be received
            tokio::select! {
                result = timeout_future => {
                    match result {
                        Ok(result) => result,
                        Err(_) => InterpretationResult::Error(InterpreterError::timeout_error("Interpreter timed out after 30 seconds")),
                    }
                }
                _ = cancel_future => {
                    InterpretationResult::Error(InterpreterError::cancellation_error("Interpreter was cancelled"))
                }
            }
        });

        // Wait for the task to complete
        let result = match handle.await {
            Ok(result) => result,
            Err(e) => InterpretationResult::Error(InterpreterError::other_error(format!(
                "Task error: {}",
                e
            ))),
        };

        // Remove the process from the map
        let mut processes = match self.processes.lock() {
            Ok(guard) => guard,
            Err(e) => {
                return InterpretationResult::Error(InterpreterError::other_error(format!(
                    "Failed to lock processes: {}",
                    e
                )))
            }
        };
        processes.remove(&pid);

        result
    }

    /// List all running processes
    /// Returns a vector of tuples containing the process ID and the code being executed
    /// This implementation returns the actual list of running processes managed by this provider
    fn list_processes(&self) -> Result<Vec<(usize, String)>> {
        let processes = self
            .processes
            .lock()
            .map_err(|e| anyhow!("Failed to lock processes: {}", e))?;
        let mut result = Vec::new();
        for (pid, info) in processes.iter() {
            result.push((*pid, info.code.clone()));
        }
        Ok(result)
    }

    /// Kill a process by ID
    /// Returns true if the process was killed, false if it wasn't found
    /// This implementation sends a cancellation signal to the process and removes it from the process map
    fn kill_process(&self, pid: usize) -> Result<bool> {
        let mut processes = self
            .processes
            .lock()
            .map_err(|e| anyhow!("Failed to lock processes: {}", e))?;
        if let Some(mut info) = processes.remove(&pid) {
            // Send cancellation signal if the sender is still available
            if let Some(sender) = info.cancel_sender.take() {
                let _ = sender.send(());
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Kill all running processes
    /// Returns the number of processes that were killed
    /// This implementation sends cancellation signals to all processes and removes them from the process map
    fn kill_all_processes(&self) -> Result<usize> {
        let mut processes = self
            .processes
            .lock()
            .map_err(|e| anyhow!("Failed to lock processes: {}", e))?;
        let count = processes.len();
        for (_, mut info) in processes.drain() {
            // Send cancellation signal if the sender is still available
            if let Some(sender) = info.cancel_sender.take() {
                let _ = sender.send(());
            }
        }
        Ok(count)
    }
}
