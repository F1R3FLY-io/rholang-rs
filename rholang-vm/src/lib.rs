// New minimal Rholang VM scaffold based on rholang-bytecode
// This crate is structured into modules: value, process, vm, and execute.

mod value;
mod process;
mod vm;
mod execute;
mod error;

pub use crate::vm::VM;
pub use crate::error::ExecError;

// Re-export a lightweight API for users
pub mod api {
    pub use rholang_bytecode::core::instructions::Instruction;
    pub use rholang_bytecode::core::opcodes::Opcode;
    pub use crate::value::Value;
    pub use crate::process::Process;
    pub use crate::vm::VM;
}
