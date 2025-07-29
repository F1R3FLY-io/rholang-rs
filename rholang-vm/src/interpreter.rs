// Rholang VM Interpreter Provider
// Integrates the VM with the shell through the InterpreterProvider interface

use crate::bytecode::{Instruction, Value};
use crate::compiler::RholangCompiler;
use crate::vm::VM;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tokio::task;
use tokio::time::Duration;

/// The result of an interpretation operation
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

    /// Create a new compilation error
    pub fn compilation_error(message: impl Into<String>) -> Self {
        InterpreterError {
            message: message.into(),
            position: None,
            source: None,
        }
    }

    /// Create a new execution error
    pub fn execution_error(message: impl Into<String>) -> Self {
        InterpreterError {
            message: message.into(),
            position: None,
            source: None,
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

impl std::fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

/// Information about a running interpreter process
struct ProcessInfo {
    /// The code being interpreted
    code: String,
    /// The cancel sender to abort the process
    cancel_sender: Option<oneshot::Sender<()>>,
}

/// Provider for the Rholang VM
/// This implements the InterpreterProvider trait
#[derive(Clone)]
pub struct RholangVMInterpreterProvider {
    /// Map of process ID to process information
    processes: Arc<Mutex<HashMap<usize, ProcessInfo>>>,
    /// Next process ID to assign
    next_pid: Arc<Mutex<usize>>,
    /// Delay for async interpretation (in milliseconds)
    delay_ms: Arc<Mutex<u64>>,
    /// Rholang bytecode compiler
    compiler: Arc<Mutex<RholangCompiler>>,
}

impl RholangVMInterpreterProvider {
    /// Create a new instance of the Rholang VM interpreter provider
    pub fn new() -> Result<Self> {
        Ok(RholangVMInterpreterProvider {
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_pid: Arc::new(Mutex::new(1)),
            delay_ms: Arc::new(Mutex::new(0)), // Default delay: 0 seconds
            compiler: Arc::new(Mutex::new(RholangCompiler::new())),
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

    /// Compile Rholang code to bytecode
    fn compile(&self, code: &str) -> Result<Vec<Instruction>> {
        // Get the compiler
        let mut compiler = self
            .compiler
            .lock()
            .map_err(|e| anyhow!("Failed to lock compiler: {}", e))?;

        // Compile the code
        compiler.compile(code)
    }
}

/// Implementation of the InterpreterProvider trait for the Rholang VM
#[async_trait]
impl InterpreterProvider for RholangVMInterpreterProvider {
    async fn interpret(&self, code: &str) -> InterpretationResult {
        // Clone the code for the process info
        let code_clone = code.to_string();

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

        // Compile the code to bytecode before spawning the task
        let bytecode = match self.compile(code) {
            Ok(bytecode) => bytecode,
            Err(e) => {
                return InterpretationResult::Error(InterpreterError::compilation_error(
                    format!("Failed to compile code: {}", e),
                ));
            }
        };

        // Spawn a task to run the VM asynchronously
        let handle = task::spawn(async move {
            // Create a future that completes when the cancel signal is received
            let cancel_future = cancel_receiver;

            // Create a future that completes when the VM finishes
            let interpret_future = async {
                // Add a delay to simulate processing time
                if delay > 0 {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }

                // Create a new VM instance
                let vm = match VM::new() {
                    Ok(vm) => vm,
                    Err(e) => {
                        return InterpretationResult::Error(InterpreterError::execution_error(
                            format!("Failed to create VM: {}", e),
                        ));
                    }
                };

                // Execute the bytecode
                match vm.execute(&bytecode).await {
                    Ok(result) => InterpretationResult::Success(result),
                    Err(e) => InterpretationResult::Error(InterpreterError::execution_error(
                        format!("Failed to execute bytecode: {}", e),
                    )),
                }
            };

            // Run the VM with a timeout
            let timeout_future = tokio::time::timeout(Duration::from_secs(30), interpret_future);

            // Wait for either the VM to finish, the timeout to expire, or the cancel signal to be received
            tokio::select! {
                result = timeout_future => {
                    result.unwrap_or_else(|_| InterpretationResult::Error(InterpreterError::timeout_error("VM timed out after 30 seconds")))
                }
                _ = cancel_future => {
                    InterpretationResult::Error(InterpreterError::cancellation_error("VM was cancelled"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vm_interpreter_provider() -> Result<()> {
        let provider = RholangVMInterpreterProvider::new()?;
        let result = provider.interpret("1 + 2").await;
        
        // The VM is not fully implemented yet, so we expect an error
        // about the Eval instruction not being implemented
        assert!(result.is_error());
        let error = result.unwrap_err();
        assert!(error.message.contains("Eval not implemented yet"));
        
        Ok(())
    }

    #[tokio::test]
    async fn test_list_processes() -> Result<()> {
        let provider = RholangVMInterpreterProvider::new()?;

        // Start a long-running process
        let code = "for(_ <- @\"channel\") { Nil }";
        let _result = provider.interpret(code).await;

        // List processes
        let processes = provider.list_processes()?;
        assert_eq!(processes.len(), 0); // The process should have completed

        Ok(())
    }

    #[tokio::test]
    async fn test_kill_process() -> Result<()> {
        let provider = RholangVMInterpreterProvider::new()?;

        // Start a long-running process
        let code = "for(_ <- @\"channel\") { Nil }";
        let _result = provider.interpret(code).await;

        // Kill a non-existent process
        let killed = provider.kill_process(999)?;
        assert!(!killed);

        Ok(())
    }

    #[tokio::test]
    async fn test_kill_all_processes() -> Result<()> {
        let provider = RholangVMInterpreterProvider::new()?;

        // Start a long-running process
        let code = "for(_ <- @\"channel\") { Nil }";
        let _result = provider.interpret(code).await;

        // Kill all processes
        let count = provider.kill_all_processes()?;
        assert_eq!(count, 0); // The process should have completed

        Ok(())
    }
}
