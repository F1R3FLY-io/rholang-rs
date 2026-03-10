//! Rholang VM execution engine.
//!
//! This crate provides the VM and step function for executing Rholang bytecode.
//! Core types (RSpace, Value, Entry, etc.) are re-exported from `rholang-rspace`.
//!
//! # Architecture
//!
//! ```text
//! rholang-rspace (core types)
//!     ↑
//! rholang-vm (execution engine) ← YOU ARE HERE
//!     ↑
//! rholang-process (process management)
//! ```

mod execute;
mod vm;

// Re-export core types from rholang-rspace
pub use rholang_rspace::{
    Entry, ExecError, InMemoryRSpace, ProcessHolder, ProcessState, RSpace, SharedRSpace, Value,
};

// Export VM and execution
pub use crate::execute::{step, StepResult};
pub use crate::vm::VM;

// Re-export a lightweight API for users
pub mod api {
    pub use crate::vm::VM;
    pub use rholang_bytecode::core::instructions::Instruction;
    pub use rholang_bytecode::core::opcodes::Opcode;
    pub use rholang_rspace::{Entry, ProcessHolder, Value};
}
