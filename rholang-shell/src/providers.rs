use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rholang_parser::{errors::ParseResult, RholangParser};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task;
use tokio::time::timeout;

/// Represents an error that occurred during interpretation
#[derive(Debug, Clone)]
pub struct InterpreterError {
    /// A human-readable error message
    pub message: String,
    /// The position in the source code where the error occurred (if available)
    pub position: Option<String>,
    /// The source code that caused the error (if available)
    pub source: Option<String>,
}

impl InterpreterError {
    /// Create a new parsing error
    pub fn parsing_error(
        message: impl Into<String>,
        position: Option<String>,
        source: Option<String>,
    ) -> Self {
        InterpreterError {
            message: message.into(),
            position,
            source,
        }
    }

    /// Create a new timeout error
    pub fn timeout_error(message: impl Into<String>) -> Self {
        InterpreterError {
            message: message.into(),
            position: None,
            source: None,
        }
    }

    /// Create a new cancellation error
    pub fn cancellation_error(message: impl Into<String>) -> Self {
        InterpreterError {
            message: message.into(),
            position: None,
            source: None,
        }
    }

    /// Create a new other error
    pub fn other_error(message: impl Into<String>) -> Self {
        InterpreterError {
            message: message.into(),
            position: None,
            source: None,
        }
    }
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;

        if let Some(position) = &self.position {
            write!(f, " at {}", position)?;
        }

        if let Some(source) = &self.source {
            write!(f, "\nSource: {}", source)?;
        }

        Ok(())
    }
}

/// Represents the result of an interpretation operation
#[derive(Debug, Clone)]
pub enum InterpretationResult {
    /// Successful interpretation with a result value
    Success(String),
    /// Error during interpretation
    Error(InterpreterError),
}

impl InterpretationResult {
    /// Returns true if the result is a success
    pub fn is_success(&self) -> bool {
        matches!(self, InterpretationResult::Success(_))
    }

    /// Returns true if the result is an error
    pub fn is_error(&self) -> bool {
        matches!(self, InterpretationResult::Error(_))
    }

    /// Unwraps the success value, panics if the result is an error
    pub fn unwrap(self) -> String {
        match self {
            InterpretationResult::Success(value) => value,
            InterpretationResult::Error(err) => panic!("Called unwrap on an error result: {}", err),
        }
    }

    /// Unwraps the error value, panics if the result is a success
    pub fn unwrap_err(self) -> InterpreterError {
        match self {
            InterpretationResult::Success(_) => {
                panic!("Called unwrap_err on a success result")
            }
            InterpretationResult::Error(err) => err,
        }
    }
}

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

/// Provider for the Rholang parser
/// This implements the InterpreterProvider trait
#[derive(Clone)]
pub struct RholangParserInterpreterProvider {
    /// Map of process ID to process information
    processes: Arc<Mutex<HashMap<usize, ProcessInfo>>>,
    /// Next process ID to assign
    next_pid: Arc<Mutex<usize>>,
    /// Delay for async interpretation (in milliseconds)
    delay_ms: Arc<Mutex<u64>>,
}

impl RholangParserInterpreterProvider {
    /// Create a new instance of the Rholang parser interpreter provider
    pub fn new() -> Result<Self> {
        Ok(RholangParserInterpreterProvider {
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_pid: Arc::new(Mutex::new(1)),
            delay_ms: Arc::new(Mutex::new(0)), // Default delay: 0 seconds
        })
    }

    /// Set the delay for async interpretation
    pub fn set_delay(&self, delay_ms: u64) -> Result<&Self> {
        let mut delay = self
            .delay_ms
            .lock()
            .map_err(|e| anyhow!("Failed to lock delay_ms: {}", e))?;
        *delay = delay_ms;
        Ok(self)
    }
}

/// Implementation of the InterpreterProvider trait for the Rholang parser
#[async_trait]
impl InterpreterProvider for RholangParserInterpreterProvider {
    async fn interpret(&self, code: &str) -> InterpretationResult {
        // Create a new parser for each call to avoid mutability issues
        let mut parser = match RholangParser::new() {
            Ok(parser) => parser,
            Err(e) => {
                return InterpretationResult::Error(InterpreterError::other_error(format!(
                    "Failed to create parser: {}",
                    e
                )))
            }
        };

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

        // Get the delay for the interpreter
        let delay = match self.delay_ms.lock() {
            Ok(guard) => *guard,
            Err(e) => {
                return InterpretationResult::Error(InterpreterError::other_error(format!(
                    "Failed to lock delay_ms: {}",
                    e
                )))
            }
        };

        // Spawn a task to run the parser asynchronously
        let handle = task::spawn(async move {
            // Create a future that completes when the cancel signal is received
            let cancel_future = cancel_receiver;

            // Create a future that completes when the parser finishes
            let interpret_future = async {
                // Add a delay to simulate processing time
                if delay > 0 {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }

                // Parse the code and return the result
                match parser.get_pretty_tree(&code_for_task) {
                    ParseResult::Success(tree_string) => InterpretationResult::Success(tree_string),
                    ParseResult::Error(err) => {
                        let position = err.position.map(|pos| format!("{}", pos));
                        InterpretationResult::Error(InterpreterError::parsing_error(
                            err.message,
                            position,
                            err.source,
                        ))
                    }
                }
            };

            // Run the parser with a timeout
            let timeout_future = timeout(Duration::from_secs(30), interpret_future);

            // Wait for either the parser to finish, the timeout to expire, or the cancel signal to be received
            tokio::select! {
                result = timeout_future => {
                    result.unwrap_or_else(|_| InterpretationResult::Error(InterpreterError::timeout_error("Parser timed out after 30 seconds")))
                }
                _ = cancel_future => {
                    InterpretationResult::Error(InterpreterError::cancellation_error("Parser was cancelled"))
                }
            }
        });

        // Wait for the task to complete
        let result = handle.await.unwrap_or_else(|e| {
            InterpretationResult::Error(InterpreterError::other_error(format!("Task error: {}", e)))
        });

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
