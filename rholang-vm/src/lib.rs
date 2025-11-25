// New minimal Rholang VM scaffold based on rholang-bytecode
// This crate is structured into modules: value, process, vm, and execute.

mod error;
mod execute;
mod process;
mod rspace;
mod value;
mod vm;

#[cfg(feature = "parallel-exec")]
pub mod parallel;

pub use crate::error::ExecError;
pub use crate::vm::VM;

// Re-export a lightweight API for users
pub mod api {
    #[cfg(feature = "parallel-exec")]
    pub use crate::parallel::vm_parallel::VmBuilder;
    #[cfg(feature = "parallel-exec")]
    pub use crate::parallel::vm_parallel::VmParallel;
    pub use crate::process::Process;
    pub use crate::value::Value;
    pub use crate::vm::VM;
    pub use rholang_bytecode::core::instructions::Instruction;
    pub use rholang_bytecode::core::opcodes::Opcode;
}
