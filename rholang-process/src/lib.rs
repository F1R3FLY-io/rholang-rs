//! Process management for Rholang execution.
//!
//! This crate defines the Process struct and execution utilities.
//! It depends on rholang-vm for VM and rholang-rspace for core types.
//!
//! # Architecture
//!
//! ```text
//! rholang-rspace (core types: RSpace, Value, Entry, ProcessState)
//!     ↑
//! rholang-vm (execution: VM, step function)
//!     ↑
//! rholang-process (process management) ← YOU ARE HERE
//! ```

mod parameter;
mod process;

pub use parameter::Parameter;
pub use process::{Process, ProcessEvent, ProcessEventHandler};

// Re-export from rholang-rspace for convenience
pub use rholang_rspace::{
    Entry, ExecError, InMemoryRSpace, ProcessHolder, ProcessState, RSpace, SharedRSpace, Value,
};

// Re-export from rholang-vm for convenience
pub use rholang_vm::{step, StepResult, VM};

use anyhow::Result;

/// Execute ready processes in parallel, updating state and emitting events.
///
/// Returns the updated processes and a list of per-process results.
pub fn execute_ready_processes(
    processes: Vec<Process>,
    handler: Option<ProcessEventHandler>,
) -> (Vec<Process>, Vec<Result<Value, ExecError>>) {
    let mut handles = Vec::with_capacity(processes.len());

    for mut process in processes {
        let handler = handler.clone();
        let handle = std::thread::spawn(move || {
            let result = if process.is_ready() {
                process.execute_with_event(handler.as_ref())
            } else {
                Ok(Value::Nil)
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

// Re-export a lightweight API for users
pub mod api {
    pub use crate::process::{Process, ProcessEvent, ProcessEventHandler};
    pub use rholang_rspace::{Entry, ProcessHolder, ProcessState, Value};
    pub use rholang_vm::api::{Instruction, Opcode, VM};
}
