// New minimal Rholang VM scaffold based on rholang-bytecode
// This crate is structured into modules: value, process, vm, and execute.

mod error;
mod execute;
mod process;
mod value;
mod vm;

pub use crate::error::ExecError;
pub use crate::execute::{step, StepResult};
pub use crate::process::{Process, ProcessEvent, ProcessEventHandler, ProcessState};
pub use crate::value::Value;
pub use crate::vm::VM;

// Re-export a lightweight API for users
pub mod api {
    pub use crate::process::Process;
    pub use crate::process::{ProcessEvent, ProcessEventHandler, ProcessState};
    pub use crate::value::Value;
    pub use crate::vm::VM;
    pub use rholang_bytecode::core::instructions::Instruction;
    pub use rholang_bytecode::core::opcodes::Opcode;
}

use anyhow::Result;

/// Execute ready processes in parallel, updating state and emitting events.
///
/// Returns the updated processes and a list of per-process results.
pub fn execute_ready_processes(
    processes: Vec<api::Process>,
    handler: Option<ProcessEventHandler>,
) -> (Vec<api::Process>, Vec<Result<api::Value, ExecError>>) {
    let handler = handler;
    let mut handles = Vec::with_capacity(processes.len());

    for mut process in processes {
        let handler = handler.clone();
        let handle = std::thread::spawn(move || {
            let result = if process.is_ready() {
                process.execute_with_event(handler.as_ref())
            } else {
                Ok(api::Value::Nil)
            };
            (process, result)
        });
        handles.push(handle);
    }

    let mut updated = Vec::new();
    let mut results = Vec::new();
    for handle in handles {
        if let Ok((process, result)) = handle.join() {
            updated.push(process);
            results.push(result);
        }
    }

    (updated, results)
}

// Minimal abstract interface for different RSpace implementations used by the current VM
pub trait RSpace: Send + Sync {
    // Put data into a channel queue (append), return true-like confirmation via Bool(true) at opcode level
    fn tell(&mut self, kind: u16, channel: String, data: Value) -> Result<()>;
    // Destructive read: remove and return oldest value, or Nil if empty / missing
    fn ask(&mut self, kind: u16, channel: String) -> Result<Option<Value>>;
    // Non-destructive read: return oldest value without removing
    fn peek(&self, kind: u16, channel: String) -> Result<Option<Value>>;
    // Reset storage (used by tests)
    fn reset(&mut self);
}
